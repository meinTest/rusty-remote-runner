use crate::process::{process, working_directory};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, get_service, post};
use axum::{Json, Router};
use rusty_runner_api::api::{
    InfoResponse, OsType, RunRequest, RunResponse, RunScriptQuery, RunStatus, ScriptInterpreter,
    VERSION,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use tower_http::services::ServeDir;

// Sanity check that our conditional compilation won't break with weird error messages.
#[cfg(all(windows, unix))]
compile_error!("Unix and Windows are exclusive!");
#[cfg(not(any(windows, unix)))]
compile_error!("Either Unix or Windows must be targeted!");

#[derive(Debug, Clone)]
struct Config {
    bash_path: Option<Arc<Path>>,
    powershell_path: Option<Arc<Path>>,
}

/// Routes under `/api`.
pub fn routes(bash_path: Option<PathBuf>, powershell_path: Option<PathBuf>) -> Router {
    Router::new()
        .route("/info", get(info))
        .route("/run", post(run_command))
        .route("/runscript", post(run_script))
        .with_state(Config {
            bash_path: bash_path.map(Into::into),
            powershell_path: powershell_path.map(Into::into),
        })
        .nest_service("/file", get_service(ServeDir::new(working_directory())))
}

async fn info() -> Json<InfoResponse> {
    log::debug!("sending info");
    Json(InfoResponse {
        api_version: String::from(VERSION),
        #[cfg(windows)]
        computer_name: std::env::var("COMPUTERNAME").unwrap_or(String::from("{unknown}")),
        #[cfg(windows)]
        os_type: OsType::Windows,
        #[cfg(unix)]
        computer_name: std::env::var("HOSTNAME").unwrap_or(String::from("{unknown}")),
        #[cfg(unix)]
        os_type: OsType::Unix,
    })
}

async fn run_command(Json(request): Json<RunRequest>) -> Json<RunResponse> {
    let id = fastrand::u64(..);

    log::info!(id; "received command");
    log::debug!(id; "command: {}", request.command);
    log::debug!(id; "arguments: {:?}", request.arguments);

    let mut command = Command::new(request.command);
    command.current_dir(working_directory());
    command.args(request.arguments);

    let response = process(id, command, request.return_stdout, request.return_stderr).await;
    Json(response)
}

async fn run_script(
    State(config): State<Config>,
    Query(query): Query<RunScriptQuery>,
    script: String,
) -> Response {
    let id = fastrand::u64(..);
    let interpreter = query.interpreter;
    log::info!(id; "received script");
    log::debug!(id; "interpreter: {interpreter:?}");
    log::debug!(id; "script: {script:?}");

    let mut script_path = working_directory();
    script_path.push(format!("script_{}.{}", id, interpreter.as_extension()));
    log::debug!(id; "script path: {script_path:?}");

    if let Err(e) = tokio::fs::write(&script_path, &script).await {
        log::error!(id; "failed to write script data: {e}");
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(failure_response(id, "Failed to write script data")),
        )
            .into_response();
    }

    // FIXME: test on unix.
    let mut command = match interpreter {
        ScriptInterpreter::Bash => {
            let Some(bash) = config.bash_path else {
                log::warn!(id; "interpreter {interpreter:?} not configured");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(failure_response(id, "Bash not supported")),
                )
                    .into_response();
            };
            let mut command = Command::new(bash.as_ref());
            command.arg("--");
            command.arg(&script_path);
            // `bash -- {file}`.
            command
        }
        ScriptInterpreter::Powershell => {
            let Some(powershell) = config.powershell_path else {
                log::warn!(id; "interpreter {interpreter:?} not configured");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(failure_response(id, "Powershell not supported")),
                )
                    .into_response();
            };
            let mut command = Command::new(powershell.as_ref());
            command.arg("-File");
            command.arg(&script_path);
            // `powershell -File {file}`.
            command
        }
        ScriptInterpreter::Cmd => {
            if !cfg!(windows) {
                log::warn!(id; "Cmd script on unix");
                return (
                    StatusCode::BAD_REQUEST,
                    Json(failure_response(id, "Cmd not supported on unix")),
                )
                    .into_response();
            }
            Command::new(script_path.as_os_str())
        }
    };

    command.current_dir(working_directory());

    let response = process(id, command, query.return_stdout, query.return_stderr).await;
    Json(response).into_response()
}

fn failure_response(id: u64, reason: impl Into<String>) -> RunResponse {
    RunResponse {
        id,
        status: RunStatus::Failure {
            reason: reason.into(),
        },
    }
}
