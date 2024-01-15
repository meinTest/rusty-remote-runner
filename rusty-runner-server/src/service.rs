use std::env;

use actix_files::NamedFile;
use rand::Rng;
use rusty_runner_api::api::*;

use actix_web::{get, post, web, HttpResponse};
use tokio::process::Command;

use crate::process::{process_command, working_directory};

#[cfg(all(windows, unix))]
compile_error!("Unix and Windows are exclusive!");
#[cfg(not(any(windows, unix)))]
compile_error!("Either Unix or Windows must be targeted!");

#[get("/api/info")]
async fn info() -> HttpResponse {
    // Note: the os type is determined at compile time, since binaries are incompatible anyway.
    // The computer name may change as the binary is copied to another machine.
    HttpResponse::Ok().json(InfoResponse {
        api_version: String::from(VERSION),
        #[cfg(windows)]
        computer_name: env::var("COMPUTERNAME").unwrap_or(String::from("{unknown}")),
        #[cfg(windows)]
        os_type: OsType::Windows,
        #[cfg(unix)]
        computer_name: env::var("HOSTNAME").unwrap_or(String::from("{unknown}")),
        #[cfg(unix)]
        os_type: OsType::Unix,
    })
}

#[post("/api/run")]
async fn run_synchronous_command(request: web::Json<RunRequest>) -> HttpResponse {
    let cmd_id = rand::thread_rng().gen::<u64>();
    let request = request.into_inner();
    log::info!("Running ID={cmd_id}");
    log::debug!("ID={cmd_id} command: {}", request.command);
    log::debug!("ID={cmd_id} arguments:{:?}", request.arguments);

    let mut command = Command::new(request.command);
    command.current_dir(working_directory());
    command.args(request.arguments);

    process_command(cmd_id, command, request.return_logs).await
}

#[post("/api/runscript")]
async fn run_synchronous_script(
    query: web::Query<RunScriptQuery>,
    body: web::Bytes,
) -> HttpResponse {
    let cmd_id = rand::thread_rng().gen::<u64>();
    let interpreter = query.interpreter;
    log::info!("Running script ID={cmd_id}");
    log::debug!("ID={cmd_id} interpreter: {interpreter:?}");

    let mut script_path = working_directory();
    script_path.push(format!("script_{}.{}", cmd_id, interpreter.as_extension()));
    log::debug!("ID={cmd_id} script path: {script_path:?}");

    let script = String::from_utf8_lossy(&body);
    log::debug!("ID={cmd_id} script: {script:?}");

    if let Err(e) = std::fs::write(script_path.as_path(), script.as_bytes()) {
        log::error!("Failed to write script data: {e}");
        return HttpResponse::InternalServerError().json(RunResponse {
            id: cmd_id,
            status: RunStatus::Failure {
                reason: String::from("Failed to write script data"),
            },
        });
    }
    // TODO: executable flag & add unix support

    let mut command = match interpreter {
        ScriptInterpreter::Bash => {
            #[cfg(windows)]
            // TODO: config for bash install path
            let mut command = Command::new(r"C:\Program Files\Git\bin\bash.exe");
            #[cfg(unix)]
            let mut command = Command::new("bash");
            command.arg(script_path.as_os_str());
            command
        }
        #[cfg(windows)]
        ScriptInterpreter::Cmd | ScriptInterpreter::Powershell => {
            // File ending determines the interpreter.
            Command::new(script_path.as_os_str())
        }
        #[allow(unreachable_patterns)]
        _ => {
            log::error!("ID={cmd_id} interpreter {interpreter:?} not supported");
            return HttpResponse::BadRequest().json(RunResponse {
                id: cmd_id,
                status: RunStatus::Failure {
                    reason: String::from("Interpreter not supported"),
                },
            });
        }
    };

    command.current_dir(working_directory());

    process_command(cmd_id, command, query.return_logs).await
}

#[get("/api/file")]
async fn get_file(query: web::Query<GetFileQuery>) -> actix_web::Result<NamedFile> {
    // This is a simple static file server provided by [`actix_files`].
    let mut path = working_directory();
    path.push(&query.path);

    Ok(NamedFile::open(path)?)
}
