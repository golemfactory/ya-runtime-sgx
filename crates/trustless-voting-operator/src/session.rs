use actix::prelude::*;
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use futures::prelude::*;
use futures::FutureExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use ya_client::model::activity::ExeScriptCommand;
use yarapi::rest;
use yarapi::rest::activity::SgxActivity;
use yarapi::rest::{Activity, RunningBatch};
use std::collections::BTreeMap;

const DEFAULT_REGISTRATION_TIME: Duration = Duration::from_secs(60 * 15);
const DEFAULT_VOTING_TIME: Duration = Duration::from_secs(60 * 10);

const ENYTY_POINT: &str = "trustless-voting-mgr";

// Parses command output
fn parse_output(output: &str) -> anyhow::Result<impl Iterator<Item = &str>> {
    const STDOUT_PREFIX: &str = "stdout: ";
    let output = if output.starts_with(STDOUT_PREFIX) {
        output[STDOUT_PREFIX.len()..].trim_end()
    } else {
        output
    };
    if !output.starts_with("OK ") {
        return Err(anyhow::anyhow!("command failed: {}", output));
    }
    let mut it = output.split_whitespace().fuse();
    let _ = it.next();
    Ok(it)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct NewSession {
    pub contract: String,
    pub voting_id: String,
    #[serde(default)]
    pub min_voters: usize,
    pub registration_deadline: Option<DateTime<Utc>>,
    pub voting_deadline: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct NewVoter {
    sender: String,
    sign: String,
    session_key: String
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub contract: String,
    pub voting_id: String,
    pub manager_address: String,
    pub state: State,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionDetails {
    pub contract: String,
    pub voting_id: String,
    pub manager_address: String,
    pub state: State,
    pub tickets: BTreeMap<String, String>,
    pub credentials: ya_client::model::activity::Credentials,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub enum State {
    #[serde(rename_all = "camelCase")]
    Init {
        min_voters: usize,
        registration_deadline: DateTime<Utc>,
        voters: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    Voting {
        voting_deadline: DateTime<Utc>,
        voters: Vec<Voter>,
    },
    Report {
        voters: Vec<String>,
        votes : BTreeMap<u32, u32>,
        signature : String
    }
}

#[derive(Serialize, Clone)]
pub struct Voter {
    pub address: String,
    pub voted: bool,
}

pub async fn new_session(
    api_session: &rest::Session,
    spec: &NewSession,
    subnet: &str,
    runtime: &str,
) -> anyhow::Result<SessionInfo> {
    let deadline = Utc::now()
        + chrono::Duration::from_std(DEFAULT_REGISTRATION_TIME)?
        + chrono::Duration::from_std(DEFAULT_VOTING_TIME)?;
    let agreement =
        crate::market::create_agreement(api_session.market()?, subnet, runtime, deadline).await?;
    let activity = api_session.create_secure_activity(&agreement).await?;

    let output = {
        let batch = activity
            .exec(vec![
                rest::ExeScriptCommand::Deploy {},
                rest::ExeScriptCommand::Start { args: vec![] },
                rest::ExeScriptCommand::Run {
                    entry_point: ENYTY_POINT.to_string(),
                    args: vec![
                        "init".to_string(),
                        spec.contract.clone(),
                        spec.voting_id.clone(),
                    ],
                },
            ])
            .await?;

        let events = batch.events();
        events
            .try_filter_map(move |e| match e {
                rest::BatchEvent::StepSuccess {
                    command: rest::ExeScriptCommand::Run { .. },
                    output,
                } => {
                    log::info!("run result: {:?}", output);
                    future::ok(Some(output))
                }
                step => {
                    log::info!("step: {:?}", step);
                    future::ok(None)
                }
            })
            .next()
            .await
    };

    {
        if let Some(output) = output {
            let output = output?;
            let tail: &str = &output["stdout: ".len()..];
            let mut it = tail.split_whitespace().fuse();
            let addr = match (it.next(), it.next(), it.next()) {
                (Some("OK"), Some(addr), _) => addr,
                _ => anyhow::bail!("failed to initialize voting manager ({:?})", output),
            };
            let now = Utc::now();
            let registration_deadline =
                now + chrono::Duration::from_std(DEFAULT_REGISTRATION_TIME)?;
            let info = SessionInfo {
                contract: spec.contract.clone(),
                voting_id: spec.voting_id.clone(),
                manager_address: addr.to_string(),
                state: State::Init {
                    min_voters: spec.min_voters,
                    registration_deadline,
                    voters: vec![],
                },
            };
            let returned_info = info.clone();
            let tickets = Default::default();
            let session_ref = Session { info, tickets, activity }.start();
            let _ = manager::SessionMgr::from_registry().do_send(manager::Register(
                returned_info.manager_address.clone(),
                session_ref,
            ));
            Ok(returned_info)
        } else {
            anyhow::bail!("failed to initialize voting manager")
        }
    }
}

pub struct Session {
    info: SessionInfo,
    tickets: BTreeMap<String, String>,
    activity: SgxActivity,
}

impl Session {
    fn exec_command(
        &self,
        command: &str,
        command_args: Vec<String>,
    ) -> impl Future<Output = anyhow::Result<String>> + 'static {
        let mut args = vec![
            command.to_string(),
            self.info.contract.clone(),
            self.info.voting_id.clone(),
            self.info.manager_address.clone(),
        ];
        args.extend(command_args);
        log::debug!("calling: {:?}", args);
        let batch = self.activity.exec(vec![rest::ExeScriptCommand::Run {
            entry_point: ENYTY_POINT.to_string(),
            args,
        }]);
        let command = command.to_string();
        async move {
            let batch = batch
                .await
                .with_context(|| format!("failed to run command: {}", command))?;
            let result = batch
                .events()
                .try_next()
                .await
                .with_context(|| format!("failed to get command results: {}", command))?;

            match result {
                Some(rest::BatchEvent::StepSuccess { output, .. }) => {
                    log::debug!("result: {}", output);
                    Ok(output)
                },
                Some(rest::BatchEvent::StepFailed { message, ..}) => {
                    log::debug!("processing faild with: {}", message);
                    anyhow::bail!("unable to process {} command", command)
                }
                _ => anyhow::bail!("unable to process {} command", command),
            }
        }
    }
}

impl Actor for Session {
    type Context = Context<Self>;
}

pub struct Get;

impl Message for Get {
    type Result = SessionInfo;
}

impl Handler<Get> for Session {
    type Result = MessageResult<Get>;

    fn handle(&mut self, _msg: Get, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.info.clone())
    }
}

pub struct GetDetails;

impl Message for GetDetails {
    type Result = SessionDetails;
}

impl Handler<GetDetails> for Session {
    type Result = MessageResult<GetDetails>;

    fn handle(&mut self, _msg: GetDetails, _ctx: &mut Self::Context) -> Self::Result {
        let tickets = self.tickets.clone();
        MessageResult(SessionDetails {
            contract: self.info.contract.clone(),
            voting_id: self.info.voting_id.clone(),
            manager_address: self.info.manager_address.clone(),
            state: self.info.state.clone(),
            credentials: self.activity.credentials().unwrap(),
            tickets
        })
    }
}

pub struct Delete;

impl Message for Delete {
    type Result = Result<()>;
}

impl Handler<Delete> for Session {
    type Result = ResponseFuture<Result<()>>;

    fn handle(&mut self, _: Delete, _: &mut Self::Context) -> Self::Result {
        let f = self.activity.exec(vec![ExeScriptCommand::Terminate {}]);
        async move {
            let batch_id = f.await?;
            let wait_for_terminate = batch_id.events().for_each(|event| {
                log::info!("on drop: {:?}", event);
                future::ready(())
            });
            let _ = tokio::time::timeout(Duration::from_secs(10), wait_for_terminate).await;
            Ok(())
        }
        .boxed_local()
    }
}

impl Message for NewVoter {
    type Result = anyhow::Result<String>;
}

impl Handler<NewVoter> for Session {
    type Result = ResponseActFuture<Self, anyhow::Result<String>>;

    fn handle(&mut self, msg: NewVoter, _ctx: &mut Self::Context) -> Self::Result {
        let sender = msg.sender;
        let out = self
            .exec_command("register", vec![sender.clone(), msg.sign.clone(), msg.session_key.clone()])
            .into_actor(self)
            .then(move |output: anyhow::Result<_>, act, _ctx| {
                log::debug!("got register command result: {:?}", output);
                fut::ready((|| -> anyhow::Result<_> {
                    let output = output?;
                    match &mut act.info.state {
                        State::Init { voters, .. } => {
                            voters.push(sender.clone());
                            Ok(())
                        }
                        _ => Err(anyhow::anyhow!("invalid state for register")),
                    }?;
                    let prefix = "stdout: OK ";
                    let ticket = if output.starts_with(prefix) {
                        output[prefix.len()..].trim().to_string()
                    } else {
                        output
                    };
                    let _ = act.tickets.insert(sender, ticket.clone());
                    Ok(ticket)
                })())
            });
        Box::pin(out)
    }
}

struct Start;

impl Message for Start {
    type Result = anyhow::Result<SessionInfo>;
}

impl Handler<Start> for Session {
    type Result = ActorResponse<Self, SessionInfo, anyhow::Error>;

    fn handle(&mut self, _msg: Start, _ctx: &mut Self::Context) -> Self::Result {
        match &self.info.state {
            State::Init { .. } => true,
            _ => return ActorResponse::reply(Ok(self.info.clone())),
        };
        ActorResponse::r#async(self.exec_command("start", vec![]).into_actor(self).then(
            |output, act, _ctx| {
                fut::result((|| {
                    let output = output?;
                    let it = parse_output(&output)?;
                    // TODO
                    let voters = it
                        .map(|addr| Voter {
                            address: addr.to_string(),
                            voted: false,
                        })
                        .collect();
                    let new_state = State::Voting {
                        voting_deadline: Utc::now()
                            + chrono::Duration::from_std(DEFAULT_VOTING_TIME)?,
                        voters,
                    };
                    act.info.state = new_state;
                    Ok(act.info.clone())
                })())
            },
        ))
    }
}

struct Finish;

impl Message for Finish {
    type Result = anyhow::Result<SessionInfo>;
}


impl Handler<Finish> for Session {
    type Result = ActorResponse<Self, SessionInfo, anyhow::Error>;

    fn handle(&mut self, _msg: Finish, _ctx: &mut Self::Context) -> Self::Result {
        match &self.info.state {
            State::Voting { .. } => true,
            _ => return ActorResponse::reply(Ok(self.info.clone())),
        };
        ActorResponse::r#async(self.exec_command("report", vec![]).into_actor(self).then(
            |output, act, _ctx| {
                fut::result((|| {
                    let output = output?;
                    let mut it = parse_output(&output)?;
                    let signature = it.next().ok_or_else(|| anyhow::anyhow!("missing signature"))?.to_string();
                    let mut votes = BTreeMap::new();
                    while let (Some(key), Some(value)) = (it.next(), it.next()) {
                        let k = u32::from_str_radix(key, 16)?;
                        let v = u32::from_str_radix(value, 16)?;
                        let _ = votes.insert(k, v);
                    }
                    let new_state = State::Report {
                        voters: Default::default(),
                        votes,
                        signature
                    };
                    act.info.state = new_state;
                    Ok(act.info.clone())
                })())
            },
        ))
    }
}


pub struct NewVote(String, Vec<u8>);

impl Message for NewVote {
    type Result = anyhow::Result<Vec<u8>>;
}

impl Handler<NewVote> for Session {
    type Result = ResponseFuture<anyhow::Result<Vec<u8>>>;

    fn handle(&mut self, msg: NewVote, _ctx: &mut Self::Context) -> Self::Result {
        // TODO reject invalid state
        match &mut self.info.state {
            State::Voting { voters, .. } => {
                if let Some(vote) = voters.iter_mut().find(|v| v.address == msg.0) {
                    vote.voted = true;
                }
            }
            _ => (),
        };

        let batch = self.exec_command("vote", vec![msg.0, hex::encode(msg.1)]);
        async move {
            let raw_output = batch.await?;
            let mut output = parse_output(&raw_output)?;
            if let Some(result) = output.next() {
                Ok(hex::decode(result)?)
            } else {
                Err(anyhow::anyhow!("Invalid response: {:?}", raw_output))
            }
        }
        .boxed_local()
    }
}

mod manager;

pub async fn list_sessions() -> Result<Vec<SessionInfo>> {
    manager::SessionMgr::from_registry()
        .send(manager::List)
        .await
        .with_context(|| "failed to list sessions")
}

pub async fn delete_manager(manager_addr: String) -> actix_web::Result<()> {
    let r = manager::SessionMgr::from_registry()
        .send(manager::Delete(manager_addr))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    r.map_err(actix_web::error::ErrorInternalServerError)
}

pub async fn register_voter(
    manager_addr: String,
    voter: NewVoter,
) -> actix_web::Result<String> {
    let voting_session = manager::SessionMgr::from_registry()
        .send(manager::Get(manager_addr.clone()))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| {
            actix_web::error::ErrorNotFound(format!("voting manager {} not found", manager_addr))
        })?;
    voting_session
        .send(voter)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .map_err(actix_web::error::ErrorInternalServerError)
}

async fn get_session_actor(manager_addr: String) -> actix_web::Result<Addr<Session>> {
    Ok(manager::SessionMgr::from_registry()
        .send(manager::Get(manager_addr.clone()))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .ok_or_else(|| {
            actix_web::error::ErrorNotFound(format!("voting manager {} not found", manager_addr))
        })?)
}

pub async fn get_session_details(manager_addr: String) -> actix_web::Result<SessionDetails> {
    get_session_actor(manager_addr)
        .await?
        .send(GetDetails)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)
}

pub async fn operator_start(manager_addr: String) -> actix_web::Result<SessionInfo> {
    get_session_actor(manager_addr)
        .await?
        .send(Start)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .map_err(actix_web::error::ErrorInternalServerError)
}

pub async fn operator_finish(manager_addr: String) -> actix_web::Result<SessionInfo> {
    get_session_actor(manager_addr)
        .await?
        .send(Finish)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .map_err(actix_web::error::ErrorInternalServerError)
}


pub async fn send_vote(
    manager_addr: String,
    sender: String,
    vote: Vec<u8>,
) -> actix_web::Result<Vec<u8>> {

    get_session_actor(manager_addr)
        .await?
        .send(NewVote(sender, vote))
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?
        .map_err(|e| actix_web::error::ErrorInternalServerError(format!("{:?}", e)))
}

pub async fn delete_all_sessions() {
    log::warn!("Admin send command delete_all_sessions");
    let _ = manager::SessionMgr::from_registry()
        .send(manager::Clean)
        .timeout(Duration::from_secs(30))
        .await;
}
