use taumada_runner::api::*;

use actix_web::{post, web, HttpResponse, Responder};
use tokio::process::Command;
use uuid::Uuid;

#[post("/api/run")]
async fn run_synchronous_command(request: web::Json<RunRequest>) -> impl Responder {
    let cmd_id = Uuid::new_v4();
    let request = request.into_inner();
    log::info!("Running command {} (`{}`)", cmd_id, request.command);

    let mut command = Command::new(request.command);
    command.args(request.arguments);
    let result = command.output().await;

    let response_json = match result {
        Ok(out) => {
            // FIXME: zero/one line stdout
            // FIXME: whole command?
            log::debug!("Command exited with code {}", out.status);
            log::debug!("> stdout:\n{}", String::from_utf8_lossy(&out.stdout).trim());
            log::debug!("> stderr:\n{}", String::from_utf8_lossy(&out.stderr).trim());
            RunCommandResponse::Completed(CompletionInfo {
                exit_code: out.status.code().unwrap_or(-1001),
            })
        }
        Err(e) => RunCommandResponse::Failure(FailureInfo {
            reason: e.to_string(),
        }),
    };

    HttpResponse::Ok().json(&response_json)
}

// TODO: asynchronous & asynchrnous exclusive
