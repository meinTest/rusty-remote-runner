use rusty_remote_runner::api::*;

use actix_web::{post, web, HttpResponse};
use tokio::process::Command;
use uuid::Uuid;

async fn process_command(cmd_id: Uuid, mut command: Command) -> HttpResponse {
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
                status: RunStatus::Completed(CompletionInfo {
                    exit_code: out.status.code().unwrap_or(-1001),
                }),
            }
        }
        Err(e) => {
            log::debug!("Command failed due to {:?}", e);
            RunResponse {
                id: cmd_id,
                status: RunStatus::Failure(FailureInfo {
                    reason: e.to_string(),
                }),
            }
        }
    };

    // Also wrap the failure into a 200 code, since it is usually due to program not found.
    HttpResponse::Ok().json(&response_json)
}

#[post("/api/run")]
async fn run_synchronous_command(request: web::Json<RunRequest>) -> HttpResponse {
    let cmd_id = Uuid::new_v4();
    let request = request.into_inner();
    log::info!("Running command {} (`{}`)", cmd_id, request.command);

    let mut command = Command::new(request.command);
    command.args(request.arguments);

    process_command(cmd_id, command).await
}

#[post("/api/runscript")]
async fn run_synchronous_script(
    query: web::Query<RunScriptQuery>,
    body: web::Bytes,
) -> HttpResponse {
    let cmd_id = Uuid::new_v4();
    let interpreter = query.interpreter;

    let mut script_path = std::env::temp_dir();
    script_path.push("rusty-runner");
    script_path.push(format!("script_{}.{}", cmd_id, interpreter.as_extension()));

    let script = String::from_utf8_lossy(&body);
    if let Err(e) = std::fs::write(script_path.as_path(), script.as_bytes()) {
        log::error!("Failed to write script data: {e}");
        return HttpResponse::InternalServerError().json(RunResponse {
            id: cmd_id,
            status: RunStatus::Failure(FailureInfo {
                reason: String::from("Failed to write script data"),
            }),
        });
    }

    let command = match interpreter {
        #[cfg(windows)]
        RunScriptInterpreter::Bash => {
            let mut command = Command::new(r"C:\Program Files\Git\bin\bash.exe");
            command.arg(script_path.as_os_str());
            command
        }
        #[cfg(windows)]
        RunScriptInterpreter::Cmd | RunScriptInterpreter::Native => {
            Command::new(script_path.as_os_str())
        }
        #[allow(unreachable_patterns)]
        _ => {
            log::error!("Interpreter {interpreter:?} not supported");
            return HttpResponse::BadRequest().json(RunResponse {
                id: cmd_id,
                status: RunStatus::Failure(FailureInfo {
                    reason: String::from("Interpreter not supported"),
                }),
            });
        }
    };

    process_command(cmd_id, command).await
}

// TODO: asynchronous & asynchrnous exclusive
