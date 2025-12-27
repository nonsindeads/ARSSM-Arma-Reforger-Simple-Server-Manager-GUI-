use axum::{http::HeaderMap, response::{Html, IntoResponse}};

pub async fn health(headers: HeaderMap) -> axum::response::Response {
    if wants_html(&headers) {
        Html(crate::views::health::health_html()).into_response()
    } else {
        "ok".into_response()
    }
}

fn wants_html(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/html"))
        .unwrap_or(false)
}
