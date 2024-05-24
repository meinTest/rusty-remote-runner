use crate::process::{process, working_directory};
use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, get_service, post, MethodRouter};
use axum::{Json, Router};
use rusty_runner_api::api::{
    InfoResponse, OsType, RunRequest, RunResponse, RunScriptQuery, RunStatus, ScriptInterpreter,
    VERSION,
};
use tokio::process::Command;
use tower_http::services::ServeDir;

pub fn routes() -> Router {
    Router::new()
        .route_logged("/info", get(info))
        .route_logged("/run", post(run_command))
        .route_logged("/runscript", post(run_script))
        .nest_service_logged("/file", get_service(ServeDir::new(working_directory())))
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

    let response = process(id, command, request.return_logs).await;
    Json(response)
}

async fn run_script(Query(query): Query<RunScriptQuery>, script: String) -> impl IntoResponse {
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
            Json(RunResponse {
                id,
                status: RunStatus::Failure {
                    reason: String::from("Failed to write script data"),
                },
            }),
        )
            .into_response();
    }

    // TODO: executable flag & add unix support
    let mut command = match interpreter {
        ScriptInterpreter::Bash => {
            #[cfg(windows)]
            // TODO: config for bash install path
            let mut command = Command::new(r"C:\Program Files\Git\bin\bash.exe");
            #[cfg(unix)]
            let mut command = Command::new("bash");
            command.arg(&script_path);
            command
        }
        #[cfg(windows)]
        ScriptInterpreter::Cmd | ScriptInterpreter::Powershell => {
            // File ending determines the interpreter.
            Command::new(script_path.as_os_str())
        }
        #[allow(unreachable_patterns)]
        _ => {
            log::error!(id; "interpreter {interpreter:?} not supported");
            return (
                StatusCode::BAD_REQUEST,
                Json(RunResponse {
                    id,
                    status: RunStatus::Failure {
                        reason: String::from("Interpreter not supported"),
                    },
                }),
            )
                .into_response();
        }
    };

    command.current_dir(working_directory());

    let response = process(id, command, query.return_logs).await;
    Json(response).into_response()
}

trait LoggedExt {
    /// [`Router::route`] and log which sub-route is being added to the router.
    fn route_logged(self, path: &str, method_router: MethodRouter) -> Self;
    /// [`Router::nest_service`] and log which nested service is being added to the router.
    fn nest_service_logged(self, path: &str, method_router: MethodRouter) -> Self;
}

impl LoggedExt for Router {
    fn route_logged(self, path: &str, method_router: MethodRouter) -> Self {
        log::info!(path; "adding sub-route");
        self.route(path, method_router)
    }

    fn nest_service_logged(self, path: &str, method_router: MethodRouter) -> Self {
        log::info!(path; "adding nested service");
        self.nest_service(path, method_router)
    }
}
