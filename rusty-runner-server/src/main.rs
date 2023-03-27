use actix_web::middleware::Logger;
use actix_web::{App, HttpServer};

mod process;
mod service;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "warn,actix_web=info,rusty_runner_server=debug");
    }
    env_logger::init();

    log::info!(
        "Starting server v{} [api v{}]",
        env!("CARGO_PKG_VERSION"),
        rusty_runner_api::api::VERSION
    );

    // Create the server working directory
    if !process::working_directory().exists() {
        std::fs::create_dir(process::working_directory())
            .expect("Should be able to write to the temporary directory!");
    }

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
    .await
}
