#![forbid(unsafe_code)]

use axum::Router;
use log::LevelFilter;
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
        .nest("/api", routes::routes())
        .layer(TraceLayer::new_for_http());

    let host = "0.0.0.0";
    let port = 1337;
    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!(
        addr:display = host,
        port = port;
        "listening to TCP"
    );

    axum::serve(listener, router.into_make_service()).await

    /*
    HttpServer::new(move || {
        App::new()
            .service(service::info)
            .service(service::run_synchronous_command)
            .service(service::run_synchronous_script)
            .service(service::get_file)
            .wrap(Logger::default())
    })
    .bind(("127.0.0.1", 8000))?
    .run()
    .await*/
}
