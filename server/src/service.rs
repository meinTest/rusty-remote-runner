use actix_files::NamedFile;
use rusty_remote_runner::api::*;

use actix_web::{get, post, web, HttpResponse};
use tokio::process::Command;
use uuid::Uuid;

use crate::process::{process_command, working_directory};

#[cfg(all(windows, unix))]
compile_error!("Unix and Windows are exclusive!");
#[cfg(not(any(windows, unix)))]
compile_error!("Either Unix and Windows must be targeted!");

// TODO: asynchronous & asynchrnous exclusive

#[get("/api/info")]
async fn info() -> HttpResponse {
    HttpResponse::Ok().json(InfoResponse {
        // FIXME: This is the server version, not api version
        api_version: String::from(env!("CARGO_PKG_VERSION")),
        #[cfg(windows)]
        computer_name: String::from(env!("COMPUTERNAME")),
        #[cfg(windows)]
        os_type: OsType::Windows,
        #[cfg(unix)]
        computer_name: String::from(env!("hostname")),
        #[cfg(unix)]
        os_type: OsType::Unix,
    })
}

#[post("/api/run")]
async fn run_synchronous_command(request: web::Json<RunRequest>) -> HttpResponse {
    let cmd_id = Uuid::new_v4();
    let request = request.into_inner();
    log::info!("Running command {} (`{}`)", cmd_id, request.command);

    let mut command = Command::new(request.command);
    command.current_dir(working_directory());
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

    let mut script_path = working_directory();
    script_path.push(format!("script_{}.{}", cmd_id, interpreter.as_extension()));

    let script = String::from_utf8_lossy(&body);
    if let Err(e) = std::fs::write(script_path.as_path(), script.as_bytes()) {
        log::error!("Failed to write script data: {e}");
        return HttpResponse::InternalServerError().json(RunResponse {
            id: cmd_id,
            status: RunStatus::Failure {
                reason: String::from("Failed to write script data"),
            },
        });
    }
    // FIXME: executable flag

    let mut command = match interpreter {
        #[cfg(windows)]
        ScriptInterpreter::Bash => {
            // TODO: config for bash install path
            let mut command = Command::new(r"C:\Program Files\Git\bin\bash.exe");
            command.arg(script_path.as_os_str());
            command
        }
        #[cfg(windows)]
        ScriptInterpreter::Cmd | ScriptInterpreter::Native => Command::new(script_path.as_os_str()),
        #[allow(unreachable_patterns)]
        _ => {
            log::error!("Interpreter {interpreter:?} not supported");
            return HttpResponse::BadRequest().json(RunResponse {
                id: cmd_id,
                status: RunStatus::Failure {
                    reason: String::from("Interpreter not supported"),
                },
            });
        }
    };

    command.current_dir(working_directory());

    process_command(cmd_id, command).await
}

#[get("/api/file")]
async fn get_file(query: web::Query<GetFileQuery>) -> actix_web::Result<NamedFile> {
    // TODO: use https://crates.io/crates/shellexpand ?

    let mut path = working_directory();
    path.push(&query.path);

    // FIXME: is this the right way to go?
    Ok(NamedFile::open(path)?)
}
