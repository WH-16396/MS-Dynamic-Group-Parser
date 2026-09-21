use axum::{routing::post, Router};
use std::net::SocketAddr;

pub mod syntax;
pub mod rules;

mod routes;

#[tokio::main]
async fn main() {
    webserver().await;
}

async fn webserver() {
    println!("Initialising router");

    let router = router();

    // Azure Functions custom handlers tell the worker which port to listen
    // on via this env var; fall back to 8080 for local `cargo run`.
    let port: u16 = std::env::var("FUNCTIONS_CUSTOMHANDLER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Router initialised, listening on port {}", port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

fn router() -> Router {
    use crate::routes::api_v1::syntax_api;

    Router::new().route("/api/v1", post(syntax_api))
}
