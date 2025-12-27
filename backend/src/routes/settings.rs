use crate::forms::{SettingsForm, SettingsQuery};
use crate::routes::AppState;
use crate::views::settings::render_settings_page;
use axum::{Form, Json, extract::State, http::StatusCode, response::Html};
use backend::defaults::parse_defaults_form;
use backend::storage::{AppSettings, load_settings, save_settings};

pub async fn settings_page(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<SettingsQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    apply_default_server_json(&mut settings);
    Ok(Html(render_settings_page(
        &settings,
        query.tab.as_deref(),
        None,
    )))
}

pub async fn settings_save(
    State(state): State<AppState>,
    Form(form): Form<SettingsForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let existing = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let mut settings = AppSettings {
        steamcmd_dir: form.steamcmd_dir,
        reforger_server_exe: form.reforger_server_exe,
        reforger_server_work_dir: form.reforger_server_work_dir,
        profile_dir_base: form.profile_dir_base,
        active_profile_id: existing.active_profile_id,
        server_json_defaults: existing.server_json_defaults,
        server_json_enabled: existing.server_json_enabled,
    };

    apply_default_server_json(&mut settings);

    if let Err(message) = settings.validate() {
        return Ok(Html(render_settings_page(
            &settings,
            Some("paths"),
            Some(&message),
        )));
    }

    save_settings(&state.settings_path, &settings)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_settings_page(
        &settings,
        Some("paths"),
        Some("Settings saved."),
    )))
}

pub async fn settings_defaults_save(
    State(state): State<AppState>,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    apply_default_server_json(&mut settings);

    let (defaults, enabled) = match parse_defaults_form(&form, &settings.server_json_defaults) {
        Ok(result) => result,
        Err(err) => {
            return Ok(Html(render_settings_page(
                &settings,
                Some("defaults"),
                Some(&err),
            )))
        }
    };
    settings.server_json_defaults = defaults;
    settings.server_json_enabled = enabled;

    save_settings(&state.settings_path, &settings)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_settings_page(
        &settings,
        Some("defaults"),
        Some("Defaults saved."),
    )))
}

pub async fn get_settings_api(
    State(state): State<AppState>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    load_settings(&state.settings_path)
        .await
        .map(Json)
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))
}

pub async fn save_settings_api(
    State(state): State<AppState>,
    Json(settings): Json<AppSettings>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    if let Err(message) = settings.validate() {
        return Err((StatusCode::BAD_REQUEST, message));
    }

    save_settings(&state.settings_path, &settings)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Json(settings))
}

#[derive(serde::Serialize)]
pub struct SteamcmdUpdateResponse {
    pub message: String,
}

pub async fn steamcmd_update() -> Json<SteamcmdUpdateResponse> {
    Json(SteamcmdUpdateResponse {
        message: "SteamCMD update placeholder executed.".to_string(),
    })
}

fn apply_default_server_json(settings: &mut AppSettings) {
    if !settings.server_json_defaults.is_object() {
        if let Ok(value) = serde_json::from_str(backend::config_gen::baseline_config()) {
            settings.server_json_defaults = value;
        }
    }
}
