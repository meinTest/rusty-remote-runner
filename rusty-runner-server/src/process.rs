use std::{path::PathBuf, time::Instant};

use rusty_runner_api::api::*;

use actix_web::HttpResponse;
use tokio::process::Command;

/// The directory where all commands will be executed in.
pub fn working_directory() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("rusty-runner");
    path
}

pub async fn process_command(cmd_id: u64, mut command: Command, return_logs: bool) -> HttpResponse {
    // Just run the command and wait for the completion.
    let start = Instant::now();
    let result = command.output().await;
    let end = Instant::now();
    let time_taken = end - start;

    let response_json = match result {
        Ok(out) => {
            // FIXME: zero/one line stdout
            log::debug!("Command {cmd_id} exited with code {}", out.status);
            log::debug!(
                "{cmd_id}> stdout:\n{}",
                String::from_utf8_lossy(&out.stdout).trim()
            );
            log::debug!(
                "{cmd_id}> stderr:\n{}",
                String::from_utf8_lossy(&out.stderr).trim()
            );
            // TODO: write logs to file ?
            RunResponse {
                id: cmd_id,
                status: RunStatus::Completed {
                    exit_code: out.status.code().unwrap_or(-1001),
                    time_taken,
                    std_out_and_err: if return_logs {
                        Some((out.stdout, out.stderr))
                    } else {
                        None
                    },
                },
            }
        }
        Err(e) => {
            log::debug!("Command {cmd_id} failed due to {:?}", e);
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
