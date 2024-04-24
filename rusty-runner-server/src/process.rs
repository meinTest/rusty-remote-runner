use std::{path::PathBuf, time::Instant};

use rusty_runner_api::api::{RunResponse, RunStatus};
use tokio::process::Command;

/// The directory where all commands will be executed in.
pub fn working_directory() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("rusty-runner");
    path
}

pub async fn process(id: u64, mut command: Command, return_logs: bool) -> RunResponse {
    // Just run the command and wait for the completion.
    let start = Instant::now();
    let result = command.output().await;
    let end = Instant::now();
    let time_taken = end - start;

    match result {
        Ok(out) => {
            // FIXME: zero/one line stdout
            log::debug!(id; "Status: {}", out.status);
            log::debug!(id; "Stdout: {}", String::from_utf8_lossy(&out.stdout).trim());
            log::debug!(id; "Stderr: {}", String::from_utf8_lossy(&out.stderr).trim());
            // TODO: write logs to file ?
            RunResponse {
                id,
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
            log::info!(id; "Failed: {e:?}");
            RunResponse {
                id,
                status: RunStatus::Failure {
                    reason: e.to_string(),
                },
            }
        }
    }
}
