//!
//! Starting code copied from `https://codevoweb.com/build-a-simple-api-with-rust-and-actix-web/`

use actix_web::middleware::Logger;
use actix_web::{post, App, HttpResponse, HttpServer, Responder};
use serde::Serialize;
use uuid::Uuid;


#[derive(Debug, Serialize)]
pub struct CommandInfo {
    id: Uuid
}

#[derive(Debug, Serialize)]
#[serde(tag = "status")]
pub enum RunCommandResponse {
    Failure,
    Pending(CommandInfo),
    Completed,
}

#[post("/api/run")]
async fn run_command() -> impl Responder {
    let cmd_id = Uuid::new_v4();
    let response_json = RunCommandResponse::Pending(CommandInfo {
        id: cmd_id
    });
    log::info!("Running command {}", cmd_id);
    HttpResponse::Ok().json(&response_json)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var(
            "RUST_LOG",
            "warn,actix_web=info,taumada_runner_server=debug",
        );
    }
    env_logger::init();

    log::info!("Starting server");

    HttpServer::new(move || App::new().service(run_command).wrap(Logger::default()))
        .bind(("127.0.0.1", 8000))?
        .run()
        .await
}
