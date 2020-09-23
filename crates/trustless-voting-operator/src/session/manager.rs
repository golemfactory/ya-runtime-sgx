use super::Session;
use crate::session::SessionInfo;
use actix::prelude::*;
use futures::prelude::*;
use futures::FutureExt;
use std::collections::BTreeMap;
use std::mem;

#[derive(Default)]
pub struct SessionMgr {
    sessions: BTreeMap<String, Addr<Session>>,
}

impl Actor for SessionMgr {
    type Context = Context<Self>;
}

impl Supervised for SessionMgr {}

impl SystemService for SessionMgr {}

pub struct Register(pub String, pub Addr<Session>);

impl Message for Register {
    type Result = ();
}

impl Handler<Register> for SessionMgr {
    type Result = ();

    fn handle(&mut self, msg: Register, _ctx: &mut Self::Context) -> Self::Result {
        let _ = self.sessions.insert(msg.0, msg.1);
    }
}

pub struct List;

impl Message for List {
    type Result = Vec<SessionInfo>;
}

impl Handler<List> for SessionMgr {
    type Result = ResponseFuture<Vec<SessionInfo>>;

    fn handle(&mut self, _msg: List, _ctx: &mut Self::Context) -> Self::Result {
        let f = future::join_all(
            self.sessions
                .iter()
                .map(|(_, session)| session.send(super::Get))
                .collect::<Vec<_>>(),
        );
        async move {
            f.await
                .into_iter()
                .filter_map(|info| match info {
                    Ok(info) => Some(info),
                    Err(_) => None,
                })
                .collect()
        }
        .boxed_local()
    }
}

pub struct Get(pub String);

impl Message for Get {
    type Result = Option<Addr<Session>>;
}

impl Handler<Get> for SessionMgr {
    type Result = MessageResult<Get>;

    fn handle(&mut self, msg: Get, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.sessions.get(&msg.0).map(|a| a.clone()))
    }
}

pub struct Delete(pub String);

impl Message for Delete {
    type Result = anyhow::Result<()>;
}

impl Handler<Delete> for SessionMgr {
    type Result = ResponseFuture<anyhow::Result<()>>;

    fn handle(&mut self, msg: Delete, _ctx: &mut Self::Context) -> Self::Result {
        let addr = self.sessions.remove(&msg.0);
        async move {
            if let Some(addr) = addr {
                // Ignore DELETE error
                let _ = addr.send(super::Delete).await;
                Ok(())
            } else {
                anyhow::bail!("manager {} not found", msg.0)
            }
        }
        .boxed_local()
    }
}

pub struct Clean;

impl Message for Clean {
    type Result = ();
}

impl Handler<Clean> for SessionMgr {
    type Result = ResponseFuture<()>;

    fn handle(&mut self, _: Clean, _: &mut Self::Context) -> Self::Result {
        let sessions = mem::replace(&mut self.sessions, Default::default());
        async move {
            let _results = future::join_all(
                sessions
                    .values()
                    .filter(|a| a.connected())
                    .map(|a| a.send(super::Delete)),
            )
            .await;
        }
        .boxed_local()
    }
}
