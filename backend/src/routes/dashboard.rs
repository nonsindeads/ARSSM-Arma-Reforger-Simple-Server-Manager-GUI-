use crate::routes::AppState;
use crate::services::{current_datetime, format_duration};
use crate::views::dashboard::{render_dashboard_page, render_server_status_card};
use crate::views::layout::template_env;
use axum::{Form, extract::State, http::StatusCode, response::Html};
use backend::storage::{list_profiles, load_packages, load_settings};
use minijinja::context;
use serde::Deserialize;

pub async fn dashboard_page(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let settings_status = if settings.validate().is_ok() {
        "Configured"
    } else {
        "Not configured"
    };

    Ok(Html(render_dashboard_page(
        profiles.len(),
        packages.len(),
        settings_status,
    )))
}

pub async fn header_status_partial(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let status = state.run_manager.status().await;
    let datetime = current_datetime();
    let uptime = status
        .started_at
        .map(|secs| format_duration(secs))
        .unwrap_or_else(|| "n/a".to_string());
    let run_status = if status.running {
        format!("running ({})", status.profile_id.unwrap_or_else(|| "unknown".to_string()))
    } else {
        "stopped".to_string()
    };
    let status_class = if status.running {
        "status-pill status-pill--running"
    } else {
        "status-pill status-pill--stopped"
    };

    let context = context! {
        datetime => datetime,
        run_status => run_status,
        status_class => status_class,
        uptime => uptime,
        cpu => "n/a",
        ram => "n/a",
    };

    let html = template_env()
        .get_template("partials/header_status.html")
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?
        .render(context)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    Ok(Html(html))
}

pub async fn server_status_card(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let status = state.run_manager.status().await;
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let active_name = crate::routes::run::active_profile_name(settings.active_profile_id.as_deref()).await;
    Ok(Html(render_server_status_card(
        &status,
        active_name.as_deref(),
        None,
    )))
}

#[derive(Deserialize)]
pub(crate) struct ServerActionForm {
    action: String,
}

pub async fn server_status_action(
    State(state): State<AppState>,
    Form(form): Form<ServerActionForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut message: Option<String> = None;
    let action = form.action.trim();
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let active_id = settings.active_profile_id.clone();

    match action {
        "start" => {
            if let Some(profile_id) = active_id.clone() {
                if let Err(err) = crate::routes::run::start_profile(&state, &settings, &profile_id).await {
                    message = Some(err);
                }
            } else {
                message = Some("No active profile configured.".to_string());
            }
        }
        "stop" => {
            let _ = state.run_manager.stop().await;
        }
        "restart" => {
            let _ = state.run_manager.stop().await;
            if let Some(profile_id) = active_id.clone() {
                if let Err(err) = crate::routes::run::start_profile(&state, &settings, &profile_id).await {
                    message = Some(err);
                }
            } else {
                message = Some("No active profile configured.".to_string());
            }
        }
        _ => {
            message = Some("Unknown action.".to_string());
        }
    }

    let status = state.run_manager.status().await;
    let active_name = crate::routes::run::active_profile_name(active_id.as_deref()).await;
    Ok(Html(render_server_status_card(
        &status,
        active_name.as_deref(),
        message.as_deref(),
    )))
}
