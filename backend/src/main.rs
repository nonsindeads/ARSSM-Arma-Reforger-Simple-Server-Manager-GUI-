mod forms;
mod routes;
mod security;
mod services;
mod views;

use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let state = routes::default_state().await;
    let app = routes::build_router(state);

    let cert_path = security::cert_path();
    let key_path = security::key_path();
    security::ensure_tls_cert(&cert_path, &key_path)
        .await
        .expect("failed to prepare TLS certificates");
    let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path)
        .await
        .expect("failed to load TLS certificates");

    let addr = "0.0.0.0:3000".parse().expect("invalid bind address");
    info!("server listening on https://0.0.0.0:3000");
    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .expect("server failed");
}
