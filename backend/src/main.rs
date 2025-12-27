mod forms;
mod routes;
mod services;
mod views;

use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let state = routes::default_state();
    let app = routes::build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind server listener");

    info!("server listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("server failed");
}
