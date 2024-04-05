#![forbid(unsafe_code)]

use axum::Router;
use tower_http::trace::TraceLayer;

mod process;
mod routes;

#[tokio::main(flavor = "current_thread")] // single-threaded, multi requires rt-multi-thread feature
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var(
            "RUST_LOG",
            "warn,tower_http=trace,rusty_runner_server=debug",
        );
    }
    env_logger::init();

    log::info!(
        "Starting server v{} [api v{}]",
        env!("CARGO_PKG_VERSION"),
        rusty_runner_api::api::VERSION
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

    let addr = "127.0.0.1:8000";
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    log::info!("Listening on {addr}");

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
