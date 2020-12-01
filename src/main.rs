use futures::{
    future::{BoxFuture, FutureExt},
    lock::Mutex,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process,
    sync::Arc,
};
use structopt::StructOpt;
use tokio::{io::AsyncWriteExt, spawn};
use ya_runtime_api::{deploy, server};

#[derive(StructOpt)]
enum Commands {
    Deploy {},
    Start {},
}

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
struct CmdArgs {
    #[structopt(short, long)]
    workdir: PathBuf,
    #[structopt(short, long)]
    task_package: PathBuf,
    #[structopt(subcommand)]
    command: Commands,
}

struct Runtime {
    work_dir: PathBuf,
    children: Arc<Mutex<Vec<process::Child>>>,
}

fn child_watcher<'a, E: server::RuntimeEvent + Send + Sync + 'static>(
    event_emitter: E,
    children: Arc<Mutex<Vec<process::Child>>>,
) -> BoxFuture<'a, ()> {
    async move {
        loop {
            /* XXX: This loop with active polling + sleep is ugly and needs to be changed
             * e.g. to `waitid` with `WNOWAIT`, once Graphene implements this syscall. */
            let mut children = children.lock().await;
            let mut found = None;
            for (i, child) in children.iter_mut().enumerate() {
                // TODO: expect("non-blocking wait for a child failed"), but this requires handling
                // errors from the spawned process
                if let Some(st) = child.try_wait().ok().flatten() {
                    found = Some((i, st));
                    break;
                }
            }
            let (child, st) = match found {
                Some((i, st)) => (children.remove(i), st),
                None => {
                    /* Drop the lock before sleeping. */
                    drop(children);
                    tokio::time::delay_for(std::time::Duration::from_millis(100)).await;
                    continue;
                }
            };
            drop(children);

            let pid = child.id();
            let mut out = Vec::new();
            let mut err = Vec::new();
            if let Some(mut stdout) = child.stdout {
                let _ = stdout.read_to_end(&mut out);
            }
            if let Some(mut stderr) = child.stderr {
                let _ = stderr.read_to_end(&mut err);
            }
            log::debug!("run process exit status: {:?}", st);
            let status = server::ProcessStatus {
                pid: pid.into(),
                running: false,
                // TODO: handle getting killed by signal
                return_code: st.code().unwrap_or(1),
                stdout: out,
                stderr: err,
            };
            event_emitter.on_process_status(status);
        }
    }
    .boxed()
}

async fn deploy<P: AsRef<Path>>(_task_package: P) -> std::io::Result<()> {
    let res = deploy::DeployResult {
        valid: Ok(Default::default()),
        vols: vec![deploy::ContainerVolume {
            name: ".".to_string(),
            path: "".to_string(),
        }],
        start_mode: deploy::StartMode::Blocking,
    };

    let mut stdout = tokio::io::stdout();
    let json = format!("{}\n", serde_json::to_string(&res)?);

    stdout.write_all(json.as_bytes()).await?;
    stdout.flush().await?;

    Ok(())
}

impl Runtime {
    async fn new<E: server::RuntimeEvent + Send + Sync + 'static>(
        work_dir: PathBuf,
        event_emitter: E,
    ) -> std::io::Result<Self> {
        let children = Arc::new(Mutex::new(Vec::new()));
        spawn(child_watcher(event_emitter, Arc::clone(&children)));
        Ok(Self { work_dir, children })
    }
}

impl server::RuntimeService for Runtime {
    fn hello(&self, version: &str) -> server::AsyncResponse<'_, String> {
        log::info!("server version: {}", version);
        async { Ok("0.0.0-demo".to_owned()) }.boxed_local()
    }

    fn run_process(
        &self,
        run: server::RunProcess,
    ) -> server::AsyncResponse<'_, server::RunProcessResp> {
        log::debug!("run process: {:?}", run);
        async move {
            let mut command = process::Command::new(run.bin);
            /* Uncomment once this (arg0) feature is stable.
            if run.args.len() > 0 {
                command.arg0(run.args[0]);
            }
            */
            if run.args.len() > 1 {
                command.args(&run.args[1..]);
            }
            let child = command
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .current_dir(&self.work_dir)
                .spawn()
                .map_err(|e| {
                    server::ErrorResponse::msg(format!("running process failed: {}", e))
                })?;
            let pid = child.id();
            let mut children = self.children.lock().await;
            children.push(child);
            Ok(server::RunProcessResp { pid: pid.into() })
        }
        .boxed_local()
    }

    fn kill_process(&self, kill: server::KillProcess) -> server::AsyncResponse<'_, ()> {
        log::debug!("kill: {:?}", kill);
        async move {
            let mut children = self.children.lock().await;
            match children
                .iter_mut()
                .find(|child| child.id() as u64 == kill.pid)
            {
                Some(child) => child.kill().map_err(|e| {
                    server::ErrorResponse::msg(format!(
                        "killing process (pid: {}) failed: {}",
                        kill.pid, e
                    ))
                }),
                None => Err(server::ErrorResponse::msg(format!(
                    "no such process (pid: {}) to kill",
                    kill.pid
                ))),
            }
        }
        .boxed_local()
    }

    fn shutdown(&self) -> server::AsyncResponse<'_, ()> {
        log::debug!("shutdown");
        async move {
            let mut children = self.children.lock().await;
            let mut fails = Vec::new();
            for child in children.iter_mut() {
                if let Err(_) = child.kill() {
                    fails.push(child.id());
                }
            }
            // TODO: kill child_watcher, perhaps giving it some time to catch up with all the children
            // being killed
            if fails.len() > 0 {
                Err(server::ErrorResponse::msg(format!(
                    "failed to kill children: {:?}",
                    fails
                )))
            } else {
                Ok(())
            }
        }
        .boxed_local()
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let cmdargs = CmdArgs::from_args();
    match cmdargs.command {
        Commands::Deploy {} => deploy(&cmdargs.task_package).await?,
        Commands::Start {} => {
            server::run_async(|e| async {
                Runtime::new(cmdargs.workdir.clone(), e)
                    .await
                    .expect("failed to start runtime")
            })
            .await
        }
    }
    Ok(())
}
