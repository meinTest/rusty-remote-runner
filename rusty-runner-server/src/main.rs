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
        "Initializing server"
    );

    // Create the server working directory
    if !process::working_directory().exists() {
        tokio::fs::create_dir(process::working_directory())
            .await
            .expect("Should be able to write to the temporary directory!");
    }

    // Setup the service
    let router = Router::new()
        .nest("/api", routes::routes(args.bash_path, args.powershell_path))
        .route("/health", get(|| async { "OK" }))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind((args.host, args.port)).await?;
    log::info!(
        on:debug = listener.local_addr();
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
    #[arg(
        long,
        value_name = "PATH",
        value_hint = ValueHint::ExecutablePath,
        env = "RUSTY_RUNNER_POWERSHELL",
    )]
    powershell_path: Option<PathBuf>,
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
