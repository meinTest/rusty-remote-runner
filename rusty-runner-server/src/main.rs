#![forbid(unsafe_code)]

use axum::routing::get;
use axum::Router;
use clap::Parser;
use log::LevelFilter;
use tokio::signal;
use tower_http::trace::TraceLayer;

mod process;
mod routes;

#[tokio::main(flavor = "current_thread")] // single-threaded, multi requires rt-multi-thread feature
async fn main() -> std::io::Result<()> {
    env_logger::builder()
        .filter(None, LevelFilter::Warn)
        .filter(Some("tower_http"), LevelFilter::Debug)
        .filter(Some("rusty_runner_server"), LevelFilter::Debug)
        .parse_default_env()
        .init();

    let CliArgs { host, port } = CliArgs::parse();

    log::info!(
        version = env!("CARGO_PKG_VERSION"),
        api_version =rusty_runner_api::api::VERSION;
        "Initializing server"
    );

    // Create the server working directory
    if !process::working_directory().exists() {
        tokio::fs::create_dir(process::working_directory())
            .await
            .expect("Should be able to write to the temporary directory!");
    }

    let router = Router::new()
        .merge(routes::routes())
        .route("/health", get(|| async { "OK" }))
        .layer(TraceLayer::new_for_http());

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!(
        addr:display = host,
        port = port;
        "listening to TCP"
    );

    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
}

#[derive(Parser)]
struct CliArgs {
    /// The host address for the rusty-runner server.
    #[arg(long, value_name = "URI", default_value = "0.0.0.0")]
    host: String,
    /// The host port for the rusty-runner server.
    #[arg(short, long, value_name = "PORT", default_value_t = 1337)]
    port: u16,
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
        _ = ctrl_c => log::info!("received SIGINT (ctrl+c), shutting down"),
        _ = terminate => log::info!("received SIGTERM, shutting down"),
    }
}
