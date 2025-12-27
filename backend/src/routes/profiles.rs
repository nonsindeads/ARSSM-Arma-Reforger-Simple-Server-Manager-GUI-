use crate::forms::{
    EditProfileForm, NewProfileCreateForm, NewProfileResolveForm, OptionalPackagesForm,
    ProfilePathsForm, ProfileTabQuery, WorkshopSaveForm,
};
use crate::routes::AppState;
use crate::services::{
    effective_value, generate_config_for_profile, normalize_optional_path, parse_mod_ids,
    parse_scenario_ids, update_list_selection,
};
use crate::views::profiles::{
    render_config_preview, render_config_preview_partial, render_new_profile_resolve,
    render_new_profile_wizard, render_profile_detail, render_profile_edit, render_profiles_page,
    render_workshop_page, render_workshop_panel,
};
use axum::{Form, extract::{Path, State}, http::{HeaderMap, StatusCode}, response::Html};
use backend::models::ServerProfile;
use backend::storage::{
    delete_profile, generated_config_path, load_packages, load_profile, load_settings,
    list_profiles, save_profile, save_settings, settings_path,
};

pub async fn profiles_page(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profiles_page(
        &profiles,
        settings.active_profile_id.as_deref(),
        None,
    )))
}

pub async fn profile_detail(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profile_detail(
        &profile,
        settings.active_profile_id.as_deref(),
    )))
}

pub async fn edit_profile_page(
    State(_state): State<AppState>,
    Path(profile_id): Path<String>,
    axum::extract::Query(query): axum::extract::Query<ProfileTabQuery>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profile_edit(
        &profile,
        &packages,
        query.tab.as_deref(),
        None,
    )))
}

pub async fn save_profile_edit(
    State(_state): State<AppState>,
    Path(profile_id): Path<String>,
    Form(form): Form<EditProfileForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if form.display_name.trim().is_empty() || form.workshop_url.trim().is_empty() {
        return Ok(Html(render_profile_edit(
            &profile,
            &packages,
            Some("general"),
            Some("Display name and workshop URL are required."),
        )));
    }

    profile.display_name = form.display_name.trim().to_string();
    profile.workshop_url = form.workshop_url.trim().to_string();
    profile.selected_scenario_id_path = normalize_optional_path(&form.selected_scenario_id_path.unwrap_or_default());
    profile.optional_package_ids = form.optional_package_ids.clone().unwrap_or_default();
    profile.optional_mod_ids = parse_mod_ids(form.optional_mod_ids.as_deref().unwrap_or(""));

    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profile_edit(
        &profile,
        &packages,
        Some("general"),
        Some("Profile updated."),
    )))
}

pub async fn update_profile_optional_packages(
    Path(profile_id): Path<String>,
    Form(form): Form<OptionalPackagesForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    profile.optional_package_ids = update_list_selection(
        form.optional_package_ids,
        &form.action,
        &form.package_id,
    );

    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profile_edit(
        &profile,
        &packages,
        Some("general"),
        Some("Optional packages updated."),
    )))
}

pub async fn delete_profile_action(
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    delete_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let settings = load_settings(&settings_path())
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profiles_page(
        &profiles,
        settings.active_profile_id.as_deref(),
        Some("Profile deleted."),
    )))
}

pub async fn save_profile_paths(
    State(_state): State<AppState>,
    Path(profile_id): Path<String>,
    Form(form): Form<ProfilePathsForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    profile.steamcmd_dir_override = normalize_optional_path(&form.steamcmd_dir_override);
    profile.reforger_server_exe_override = normalize_optional_path(&form.reforger_server_exe_override);
    profile.reforger_server_work_dir_override = normalize_optional_path(&form.reforger_server_work_dir_override);
    profile.profile_dir_base_override = normalize_optional_path(&form.profile_dir_base_override);

    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profile_edit(
        &profile,
        &packages,
        Some("paths"),
        Some("Profile paths saved."),
    )))
}

pub async fn save_profile_overrides(
    Path(profile_id): Path<String>,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let settings = load_settings(&settings_path())
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let (overrides, enabled) = match backend::defaults::parse_defaults_form(&form, &settings.server_json_defaults) {
        Ok(result) => result,
        Err(err) => {
            return Ok(Html(render_profile_edit(
                &profile,
                &packages,
                Some("overrides"),
                Some(&err),
            )));
        }
    };
    profile.server_json_overrides = overrides;
    profile.server_json_override_enabled = enabled;

    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profile_edit(
        &profile,
        &packages,
        Some("overrides"),
        Some("Overrides saved."),
    )))
}

pub async fn new_profile_page() -> Result<Html<String>, (StatusCode, String)> {
    Ok(Html(render_new_profile_wizard(None)))
}

pub async fn new_profile_resolve(
    State(state): State<AppState>,
    Form(form): Form<NewProfileResolveForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let workshop_url = form.workshop_url.trim().to_string();
    if workshop_url.is_empty() {
        return Ok(Html(render_new_profile_wizard(Some(
            "Workshop URL is required.",
        ))));
    }

    let result = state
        .workshop_resolver
        .resolve(&workshop_url, 5)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;
    Ok(Html(render_new_profile_resolve(
        Some(&result),
        None,
    )))
}

pub async fn new_profile_create(
    Form(form): Form<NewProfileCreateForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    if form.display_name.trim().is_empty() {
        return Ok(Html(render_new_profile_wizard(Some(
            "Display name is required.",
        ))));
    }

    let scenario_ids = form
        .scenario_ids
        .as_deref()
        .map(parse_scenario_ids)
        .unwrap_or_default();
    let selected = normalize_optional_path(&form.selected_scenario_id_path.unwrap_or_default());
    let optional_mod_ids = parse_mod_ids(form.optional_mod_ids.as_deref().unwrap_or(""));
    let dependency_mod_ids = form
        .dependency_mod_ids
        .as_deref()
        .map(parse_mod_ids)
        .unwrap_or_default();

    let profile = ServerProfile {
        profile_id: new_profile_id(),
        display_name: form.display_name.trim().to_string(),
        workshop_url: form.workshop_url.trim().to_string(),
        root_mod_id: form
            .root_mod_id
            .clone()
            .and_then(|value| normalize_optional_path(&value)),
        selected_scenario_id_path: selected.clone(),
        scenarios: scenario_ids,
        dependency_mod_ids,
        optional_mod_ids,
        optional_package_ids: Vec::new(),
        load_session_save: false,
        steamcmd_dir_override: None,
        reforger_server_exe_override: None,
        reforger_server_work_dir_override: None,
        profile_dir_base_override: None,
        server_json_overrides: serde_json::json!({}),
        server_json_override_enabled: std::collections::HashMap::new(),
        generated_config_path: None,
        last_resolved_at: Some(now_timestamp()),
        last_resolve_hash: None,
    };

    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profile_detail(&profile, None)))
}

pub async fn activate_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let mut settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    if profiles.iter().any(|profile| profile.profile_id == profile_id) {
        settings.active_profile_id = Some(profile_id.clone());
    }
    save_settings(&state.settings_path, &settings)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profiles_page(
        &profiles,
        settings.active_profile_id.as_deref(),
        Some("Active profile updated."),
    )))
}

pub async fn profile_workshop_page(
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    Ok(Html(render_workshop_page(&profile, None, None)))
}

pub async fn profile_workshop_resolve(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    if profile.workshop_url.trim().is_empty() {
        return Ok(Html(render_workshop_page(
            &profile,
            None,
            Some("Workshop URL is missing."),
        )));
    }

    let result = resolve_and_update_profile(&state, &mut profile)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;

    if is_hx_request(&headers) {
        return Ok(Html(render_workshop_panel(
            &profile,
            Some(&result),
            None,
        )));
    }

    Ok(Html(render_workshop_page(&profile, Some(&result), None)))
}

pub async fn profile_workshop_save(
    Path(profile_id): Path<String>,
    Form(form): Form<WorkshopSaveForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    profile.selected_scenario_id_path = normalize_optional_path(&form.selected_scenario_id_path);
    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_workshop_page(
        &profile,
        None,
        Some("Scenario selection saved."),
    )))
}

pub async fn config_preview_page(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let preview = match generate_config_for_profile(&profile, &settings, &packages) {
        Ok(value) => serde_json::to_string_pretty(&value)
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?,
        Err(err) => err,
    };
    Ok(Html(render_config_preview(&profile, &preview, None)))
}

pub async fn config_preview_partial(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    let result = resolve_and_update_profile(&state, &mut profile)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;

    if let Err(message) = validate_selected_scenario(&profile, &result.scenarios) {
        return Ok(Html(render_config_preview_partial(
            &format!("Error: {message}"),
            Some("Resolve failed."),
        )));
    }
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let preview = match generate_config_for_profile(&profile, &settings, &packages) {
        Ok(value) => serde_json::to_string_pretty(&value)
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?,
        Err(err) => err,
    };
    let notice = if result.errors.is_empty() {
        Some("Resolved and regenerated.")
    } else {
        Some("Resolved with warnings; regenerated.")
    };

    Ok(Html(render_config_preview_partial(&preview, notice)))
}

pub async fn write_config(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    let resolve_result = resolve_and_update_profile(&state, &mut profile)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;

    if let Err(message) = validate_selected_scenario(&profile, &resolve_result.scenarios) {
        return Err((StatusCode::BAD_REQUEST, message));
    }

    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let config = generate_config_for_profile(&profile, &settings, &packages)
        .map_err(|message| (StatusCode::BAD_REQUEST, message))?;
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let server_work_dir = effective_value(
        &profile.reforger_server_work_dir_override,
        &settings.reforger_server_work_dir,
    );
    let path = generated_config_path(server_work_dir, &profile.profile_id);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;
    }
    tokio::fs::write(&path, &config_json)
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    profile.generated_config_path = Some(path.to_string_lossy().to_string());
    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let notice = if resolve_result.errors.is_empty() {
        "Config written successfully."
    } else {
        "Config written with resolve warnings."
    };

    Ok(Html(render_config_preview(
        &profile,
        &config_json,
        Some(notice),
    )))
}

pub async fn regenerate_config(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    let resolve_result = resolve_and_update_profile(&state, &mut profile)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;

    let notice = if let Err(message) = validate_selected_scenario(&profile, &resolve_result.scenarios) {
        let preview = message;
        return Ok(Html(render_config_preview(
            &profile,
            &preview,
            Some("Scenario selection invalid."),
        )));
    } else if resolve_result.errors.is_empty() {
        Some("Config regenerated.")
    } else {
        Some("Config regenerated with resolve warnings.")
    };

    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let preview = match generate_config_for_profile(&profile, &settings, &packages) {
        Ok(value) => serde_json::to_string_pretty(&value)
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?,
        Err(err) => err,
    };

    Ok(Html(render_config_preview(&profile, &preview, notice)))
}

fn is_hx_request(headers: &HeaderMap) -> bool {
    headers
        .get("HX-Request")
        .and_then(|value| value.to_str().ok())
        .map(|value| value == "true")
        .unwrap_or(false)
}

fn validate_selected_scenario(profile: &ServerProfile, scenarios: &[String]) -> Result<(), String> {
    let selected = profile
        .selected_scenario_id_path
        .as_deref()
        .ok_or_else(|| "selected_scenario_id_path not set".to_string())?;
    if scenarios.is_empty() {
        return Err("no scenarios resolved; resolve workshop first".to_string());
    }
    if !scenarios.iter().any(|value| value == selected) {
        return Err("selected scenario no longer available".to_string());
    }
    Ok(())
}

async fn resolve_and_update_profile(
    state: &AppState,
    profile: &mut ServerProfile,
) -> Result<backend::workshop::WorkshopResolveResult, String> {
    if profile.workshop_url.trim().is_empty() {
        return Err("workshop_url is missing".to_string());
    }

    let result = state.workshop_resolver.resolve(&profile.workshop_url, 5).await?;
    profile.root_mod_id = Some(result.root_id.clone());
    profile.dependency_mod_ids = result.dependency_ids.clone();
    profile.scenarios = result.scenarios.clone();
    profile.last_resolved_at = Some(now_timestamp());
    save_profile(profile).await?;
    Ok(result)
}

fn new_profile_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("profile-{nanos}")
}

fn now_timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    seconds.to_string()
}
