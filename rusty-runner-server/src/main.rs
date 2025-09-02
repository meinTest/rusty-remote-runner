//! Runs a server complying with the [`rusty_runner_api`].
//!
//! Listens on `http://localhost:8000`, e.g. `http://localhost:8000/api/info` unless changed by the [`CliArgs`].
//! The working directory is determined by [`{std::env::temp_dir()}/rusty-runner`][process::working_directory].
//!
//! The paths to bash and powershell are configured in [`CliArgs`] and must be set to support the respective interpreters.

use axum::routing::get;
use axum::Router;
use clap::{Parser, ValueHint};
use log::LevelFilter;
use std::path::PathBuf;
use tokio::signal;
use tower_http::trace::TraceLayer;

mod cleanup;
mod process;
mod routes;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .filter(Some("tower_http"), LevelFilter::Debug)
        .filter(Some("rusty_runner_server"), LevelFilter::Debug)
        .parse_default_env()
        .init();

    let args = CliArgs::parse();

    log::info!(
        version = env!("CARGO_PKG_VERSION"),
        api_version = rusty_runner_api::api::VERSION;
        "initializing server"
    );
    log::info!(path:debug = args.powershell_path; "configured powershell");
    log::info!(path:debug = args.bash_path; "configured bash");
    log::info!(path:debug = args.cleanup_max_age; "configured age-based cleanup");
    log::info!(path:debug = args.cleanup_max_size; "configured size-based cleanup");

    // Create the server working directory
    if !process::working_directory().exists() {
        tokio::fs::create_dir(process::working_directory())
            .await
            .expect("Should be able to write to the temporary directory!");
    }

    // Start cleaning up regularly
    cleanup::start_cleanup_task(args.cleanup_max_age, args.cleanup_max_size);

    // Setup the service
    let router = Router::new()
        .nest("/api", routes::routes(args.bash_path, args.powershell_path))
        .route("/health", get(|| async { "OK" }))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind((args.host, args.port)).await?;
    log::info!(
        on:debug = listener.local_addr()?;
        "listening to TCP"
    );

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
}

/// Runs a server complying with the `rusty_runner_api`.
///
/// By default listens on `http://localhost:8000`, e.g. `http://localhost:8000/api/info` unless changed by the command line arguments.
/// The working directory is determined by `{std::env::temp_dir()}/rusty-runner`.
///
/// The paths to bash and powershell must be set to support the respective interpreters.
#[derive(Parser)]
struct CliArgs {
    /// The host address for the rusty-runner server.
    /// Keep `127.0.0.1` to protect against access from the outside.
    #[arg(
        long,
        value_name = "URI",
        value_hint = ValueHint::Hostname,
        default_value = "127.0.0.1",
        env = "RUSTY_RUNNER_HOST",
    )]
    host: String,
    /// The host port for the rusty-runner server. 0 means it will pick any free port.
    #[arg(
        short,
        long,
        value_name = "PORT",
        value_hint = ValueHint::Other,
        default_value = "8000",
        env = "RUSTY_RUNNER_PORT",
    )]
    port: u16,
    /// The path of the bash interpreter. If not set, bash scripts are not supported.
    ///
    /// Can be just the name of the binary if it is in the PATH.
    /// *WARNING* on windows this misbehaves with a bare name. Use the full path, commonly `C:\Program Files\Git\bin\bash.exe` instead.
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::ExecutablePath,
        env = "RUSTY_RUNNER_BASH",
    )]
    bash_path: Option<PathBuf>,
    /// The path of the powershell interpreter. If not set, powershell scripts are not supported.
    /// Can be just the name of the binary if it is in the PATH.
    /// On unix this is often `pwsh`, while windows uses `powershell`.
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::ExecutablePath,
        env = "RUSTY_RUNNER_POWERSHELL",
    )]
    powershell_path: Option<PathBuf>,
    /// The maximum age for entries in the working directory, e.g. `1.5d` for 1.5 days.
    /// Also supported suffixes: `w` for weeks, `h` for hours.
    #[arg(
        long,
        value_name = "DAYS",
        value_hint = ValueHint::Other,
        env = "RUSTY_RUNNER_MAX_AGE",
        value_parser = parse_duration
    )]
    cleanup_max_age: Option<std::time::Duration>,
    /// The maximum size for entries in the working directory, e.g. `2.5G` or `2.5GB` for 2.5 gigabytes.
    /// Also supported suffixes: `M`/`MB` for megabytes, `T`/`TB` for terrabytes.
    ///
    /// Note that we use binary definitions of giga, i.e. 1GB = files sizes that amount to 1024^3 bytes
    #[arg(
        long,
        value_name = "GB",
        value_hint = ValueHint::Other,
        env = "RUSTY_RUNNER_MAX_SIZE",
        value_parser = parse_size
    )]
    cleanup_max_size: Option<usize>,
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install SIGINT (ctrl+c) handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => log::info!("received SIGINT (ctrl+c), shutting down"),
        () = terminate => log::info!("received SIGTERM, shutting down"),
    }
}

fn parse_suffixed_num(s: &str) -> Result<(f32, String), String> {
    let (num, unit): (String, String) = s.chars().partition(|c| !c.is_alphabetic());

    let num: f32 = num
        .trim()
        .parse()
        .map_err(|_| format!("Invalid number: {num}"))?;
    if num.is_finite() {
        Ok((num, unit))
    } else {
        Err("Number non-finite".to_string())
    }
}
fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
    let (num, unit) = parse_suffixed_num(s)?;
    if num < 0.0 {
        return Err("Duration cannot be negative".to_string());
    }
    match unit.trim().to_ascii_uppercase().as_str() {
        "H" => Ok(std::time::Duration::from_secs_f32(num * 60. * 60.)),
        "D" => Ok(std::time::Duration::from_secs_f32(num * 24. * 60. * 60.)),
        "W" => Ok(std::time::Duration::from_secs_f32(
            num * 7. * 24. * 60. * 60.,
        )),
        _ => Err(format!("Invalid unit for duration: {unit}")),
    }
}
#[allow(clippy::cast_sign_loss)] // checked against
#[allow(clippy::cast_possible_truncation)] // won't reasonably happen
fn parse_size(s: &str) -> Result<usize, String> {
    let (num, unit) = parse_suffixed_num(s)?;
    if num < 0.0 {
        return Err("Size cannot be negative".to_string());
    }
    match unit.trim().to_ascii_uppercase().as_str() {
        "M" | "MB" => Ok((num * 1024.0 * 1024.0).round() as usize),
        "G" | "GB" => Ok((num * 1024.0 * 1024.0 * 1024.0).round() as usize),
        "T" | "TB" => Ok((num * 1024.0 * 1024.0 * 1024.0 * 1024.0).round() as usize),
        _ => Err(format!("Invalid unit for size: {unit}")),
    }
}
