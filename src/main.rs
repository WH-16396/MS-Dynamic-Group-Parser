//! HTTP API for parsing, tidying, checking and rebuilding Microsoft Entra
//! dynamic group membership rules.
//!
//! A request flows through the crate in four stages:
//!
//! 1. `parser`  - rule syntax text -> `Dnf` (a flat OR-list of AND-rules)
//! 2. `process` - tidy the `Dnf`, optionally flag risky rules
//! 3. `process::reconstruct` - fold the `Dnf` back into a minimal tree
//! 4. `routes`  - serialise the tree / syntax / rules into the response
//!
//! Runs locally with `cargo run`, or as an Azure Functions custom handler
//! (see `host.json`).

use axum::{Router, routing::post};
use std::net::SocketAddr;

pub mod model;
pub mod parser;
pub mod process;

mod routes;

/// Port used when not running under Azure Functions.
const DEFAULT_PORT: u16 = 8082;

#[tokio::main]
async fn main() {
    println!("Initialising router");

    let router = router();

    // Azure Functions custom handlers tell the worker which port to listen
    // on via this env var; fall back to the default for local `cargo run`.
    let port: u16 = std::env::var("FUNCTIONS_CUSTOMHANDLER_PORT")
        .ok()
        .and_then(|port| port.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let address = SocketAddr::from(([0, 0, 0, 0], port));

    println!("Router initialised, listening on port {}", port);

    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    axum::serve(listener, router).await.unwrap();
}

fn router() -> Router {
    Router::new()
        .route("/api/v1", post(routes::v1::handler))
        .route("/api/v2", post(routes::v2::handler))
}
