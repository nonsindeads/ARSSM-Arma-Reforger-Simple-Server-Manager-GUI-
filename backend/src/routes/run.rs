use crate::forms::RunStartRequest;
use crate::routes::AppState;
use crate::services::{effective_path_value, generate_config_for_profile};
use crate::views::run::render_run_logs_page;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::Html,
};
use axum::response::sse::{Event, Sse};
use backend::runner::RunStatus;
use backend::storage::{
    generated_config_path, list_profiles, load_packages, load_profile, load_settings, save_profile,
};
use std::path::PathBuf;
use tokio_stream::{StreamExt, wrappers::BroadcastStream};

pub async fn run_logs_page() -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_run_logs_page(&profiles)))
}

pub async fn run_status(
    State(state): State<AppState>,
) -> Result<Json<RunStatus>, (StatusCode, String)> {
    Ok(Json(state.run_manager.status().await))
}

pub async fn run_start(
    State(state): State<AppState>,
    Json(request): Json<RunStartRequest>,
) -> Result<Json<RunStatus>, (StatusCode, String)> {
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if let Err(message) = settings.validate() {
        return Err((StatusCode::BAD_REQUEST, message));
    }

    let profile_id = match request.profile_id.clone().filter(|value| !value.trim().is_empty()) {
        Some(value) => value,
        None => settings
            .active_profile_id
            .clone()
            .ok_or_else(|| (StatusCode::BAD_REQUEST, "active profile not set".to_string()))?,
    };

    start_profile(&state, &settings, &profile_id)
        .await
        .map_err(|message| (StatusCode::BAD_REQUEST, message))?;

    Ok(Json(state.run_manager.status().await))
}

pub async fn run_stop(
    State(state): State<AppState>,
) -> Result<Json<RunStatus>, (StatusCode, String)> {
    state
        .run_manager
        .stop()
        .await
        .map_err(|message| (StatusCode::BAD_REQUEST, message))?;
    Ok(Json(state.run_manager.status().await))
}

#[derive(serde::Serialize)]
pub(crate) struct LogTailResponse {
    lines: Vec<String>,
}

pub async fn run_logs_tail(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<LogTailResponse>, (StatusCode, String)> {
    let limit = params
        .get("n")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(200);
    let lines = state.run_manager.tail(limit).await;
    Ok(Json(LogTailResponse { lines }))
}

pub async fn run_logs_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let receiver = state.run_manager.subscribe();
    let stream = BroadcastStream::new(receiver)
        .filter_map(|message| message.ok())
        .map(|line| Ok(Event::default().data(line)));
    Sse::new(stream)
}

pub(crate) async fn start_profile(
    state: &AppState,
    settings: &backend::storage::AppSettings,
    profile_id: &str,
) -> Result<(), String> {
    let mut profile = load_profile(profile_id).await?;

    let server_work_dir = effective_path_value(
        &profile.reforger_server_work_dir_override,
        &settings.reforger_server_work_dir,
    );
    let config_path = generated_config_path(&server_work_dir, &profile.profile_id);

    if tokio::fs::metadata(&config_path).await.is_err() {
        let packages = load_packages().await?;
        let config_value = generate_config_for_profile(&profile, settings, &packages)?;
        let config_json = serde_json::to_string_pretty(&config_value)
            .map_err(|err| format!("failed to serialize config: {err}"))?;
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|err| format!("failed to create config dir: {err}"))?;
        }
        tokio::fs::write(&config_path, &config_json)
            .await
            .map_err(|err| format!("failed to write config: {err}"))?;
        profile.generated_config_path = Some(config_path.to_string_lossy().to_string());
        save_profile(&profile).await?;
    }

    let profile_dir_base =
        effective_path_value(&profile.profile_dir_base_override, &settings.profile_dir_base);
    let profile_dir = PathBuf::from(&profile_dir_base).join(&profile.profile_id);
    let server_exe =
        effective_path_value(&profile.reforger_server_exe_override, &settings.reforger_server_exe);

    state
        .run_manager
        .start(&server_exe, &server_work_dir, &profile, &config_path, &profile_dir)
        .await
}

pub(crate) async fn active_profile_name(profile_id: Option<&str>) -> Option<String> {
    let profile_id = profile_id?;
    load_profile(profile_id).await.ok().map(|profile| profile.display_name)
}
