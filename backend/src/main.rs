use axum::{Router, routing::get, response::Html};
use tracing::info;

async fn health() -> &'static str {
    "ok"
}

async fn root() -> Html<&'static str> {
    Html(
        "<!doctype html><html><head><title>ARSSM</title></head>\
<body><h1>ARSSM</h1><p>Arma Reforger Simple Server Manager</p>\
<p><a href=\"/health\">/health</a></p></body></html>",
    )
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind server listener");

    info!("server listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("server failed");
}
