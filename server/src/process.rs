use std::path::PathBuf;

use rusty_remote_runner::api::*;

use actix_web::HttpResponse;
use tokio::process::Command;
use uuid::Uuid;

pub fn working_directory() -> PathBuf {
    // FIXME: mkdir if not existing
    let mut path = std::env::temp_dir();
    path.push("rusty-runner");
    path
}

pub async fn process_command(cmd_id: Uuid, mut command: Command) -> HttpResponse {
    // Just run the command.
    // FIXME: delay running and early return
    let result = command.output().await;

    let response_json = match result {
        Ok(out) => {
            // FIXME: zero/one line stdout
            // FIXME: whole command?
            log::debug!("Command exited with code {}", out.status);
            log::debug!("> stdout:\n{}", String::from_utf8_lossy(&out.stdout).trim());
            log::debug!("> stderr:\n{}", String::from_utf8_lossy(&out.stderr).trim());
            RunResponse {
                id: cmd_id,
                status: RunStatus::Completed {
                    exit_code: out.status.code().unwrap_or(-1001),
                },
            }
        }
        Err(e) => {
            log::debug!("Command failed due to {:?}", e);
            RunResponse {
                id: cmd_id,
                status: RunStatus::Failure {
                    reason: e.to_string(),
                },
            }
        }
    };

    // Also wrap the failure into a 200 code, since it is usually due to program not found.
    HttpResponse::Ok().json(&response_json)
}
