use crate::session::SessionInfo;
use actix_web::{
    delete, get, post, put, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};

use std::sync::Arc;

use structopt::StructOpt;
use yarapi::rest;

mod market;
mod session;

#[get("/version")]
async fn version() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "version": env!("CARGO_PKG_VERSION"), "pre": env!("CARGO_PKG_VERSION_PRE") }))
}

#[get("/nodes")]
async fn nodes(session: web::Data<rest::Session>) -> impl Responder {
    let m: rest::Market = session
        .market()
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok::<_, actix_web::Error>(
        HttpResponse::Ok().json(
            market::list_nodes(m, "sgx", "sgx")
                .await
                .map_err(actix_web::error::ErrorInternalServerError)?,
        ),
    )
}

#[post("/sessions")]
async fn new_session(
    api_session: web::Data<rest::Session>,
    args: web::Data<Arc<Args>>,
    spec: web::Json<session::NewSession>,
) -> impl Responder {
    api_session
        .with(async {
            Ok::<_, actix_web::Error>(
                HttpResponse::Ok().json(
                    session::new_session(&api_session, &spec, &args.subnet, "sgx")
                        .await
                        .map_err(actix_web::error::ErrorInternalServerError)?,
                ),
            )
        })
        .await
}

#[get("/sessions")]
async fn sessions() -> actix_web::Result<web::Json<Vec<SessionInfo>>> {
    let sessions = session::list_sessions()
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok(web::Json(sessions))
}

#[get("/sessions/{mgrAddr}")]
async fn fetch_session(msg_addr: web::Path<(String,)>) -> impl Responder {
    let session = session::get_session_details(msg_addr.into_inner().0)
        .await
        .map_err(actix_web::error::ErrorInternalServerError)?;
    Ok::<_, actix_web::Error>(web::Json(session))
}

#[delete("/sessions/{mgrAddr}")]
async fn delete_sessoion(msg_addr: web::Path<(String,)>) -> impl Responder {
    session::delete_manager(msg_addr.into_inner().0).await?;
    Ok::<_, actix_web::Error>(web::Json(()))
}

#[post("/sessions/{mgrAddr}")]
async fn register_voter(
    msg_addr: web::Path<(String,)>,
    voter: web::Json<session::NewVoter>,
) -> impl Responder {
    let session_info = session::register_voter(msg_addr.into_inner().0, voter.into_inner()).await?;
    Ok::<_, actix_web::Error>(web::Json(session_info))
}

#[put("/session/{msgAddr}/vote/{sender}")]
async fn send_vote(path: web::Path<(String, String)>, vote: web::Bytes) -> impl Responder {
    let (manager_addr, sender) = path.into_inner();
    let response = session::send_vote(manager_addr, sender, vote.as_ref().into()).await?;
    Ok::<_, actix_web::Error>(web::Bytes::from(response))
}

#[post("/admin/sessions/{mgrAddr}/start")]
async fn session_start(msg_addr: web::Path<(String,)>) -> impl Responder {
    let session_info = session::operator_start(msg_addr.into_inner().0).await?;
    Ok::<_, actix_web::Error>(web::Json(session_info))
}

#[post("/admin/shutdown")]
async fn admin_shutdown() -> impl Responder {
    session::delete_all_sessions().await;
    web::Json(())
}

#[derive(StructOpt)]
struct Args {
    #[structopt(long, default_value = "sgx", env = "SUBNET")]
    subnet: String,
    #[structopt(long, env = "YAGNA_APPKEY")]
    appkey: String,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    env_logger::from_env("TVO_LOG")
        .filter_level(log::LevelFilter::Info)
        .init();

    let args = Arc::new(Args::from_args());

    HttpServer::new(move || {
        App::new()
            .data({
                let client = yarapi::rest::WebClient::with_token(&args.appkey);
                yarapi::rest::Session::with_client(client)
            })
            .data(args.clone())
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                let msg = match &err {
                    actix_web::error::JsonPayloadError::Deserialize(e) => format!("{}", e),
                    e => format!("{:?}", e),
                };
                actix_web::error::InternalError::from_response(
                    err,
                    HttpResponse::BadRequest().body(msg),
                )
                .into()
            }))
            .service(version)
            .service(nodes)
            .service(sessions)
            .service(fetch_session)
            .service(new_session)
            .service(delete_sessoion)
            .service(register_voter)
            .service(send_vote)
            .service(session_start)
            .service(admin_shutdown)
            .service(actix_files::Files::new("/ui", "ui"))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
