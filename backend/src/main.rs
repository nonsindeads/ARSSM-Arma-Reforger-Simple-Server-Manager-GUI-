use axum::{Router, routing::get};
use tracing::info;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let web_dir = web_dir();
    let index_file = web_dir.join("index.html");

    let app = Router::new()
        .route("/health", get(health))
        .route_service("/", ServeFile::new(index_file))
        .nest_service("/web", ServeDir::new(web_dir));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind server listener");

    info!("server listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("server failed");
}

fn web_dir() -> PathBuf {
    std::env::var("ARSSM_WEB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("web"))
}
