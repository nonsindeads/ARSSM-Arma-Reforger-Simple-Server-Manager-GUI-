use axum::{
    Form, Json, Router, extract::{Path, State}, http::{HeaderMap, StatusCode}, response::{Html, IntoResponse}, routing::get
};
use backend::{
    config_gen::generate_server_config,
    models::ServerProfile,
    runner::{RunManager, RunStatus},
    storage::{
        AppSettings, delete_profile, generated_config_path, load_mods, load_packages, load_profile,
        load_settings, list_profiles, save_mods, save_packages, save_profile, save_settings,
        settings_path,
    },
    workshop::{ReqwestFetcher, WorkshopResolveRequest, WorkshopResolver},
};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::{io, path::PathBuf};
use tower_http::services::ServeDir;
use axum::response::sse::{Event, Sse};
use tokio_stream::{StreamExt, wrappers::BroadcastStream};
use minijinja::{Environment, context};
use std::sync::OnceLock;

async fn health(headers: HeaderMap) -> axum::response::Response {
    if wants_html(&headers) {
        Html(health_html()).into_response()
    } else {
        "ok".into_response()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AppConfig {
    steamcmd_dir: String,
    reforger_server_exe: String,
    reforger_server_work_dir: String,
    server_config_path: String,
    profile_dir: String,
    load_session_save: bool,
}

#[derive(Clone)]
struct AppState {
    config_path: PathBuf,
    workshop_resolver: WorkshopResolver,
    settings_path: PathBuf,
    run_manager: RunManager,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let web_dir = web_dir();
    let state = AppState {
        config_path: config_path(),
        workshop_resolver: WorkshopResolver::new(std::sync::Arc::new(ReqwestFetcher::new())),
        settings_path: settings_path(),
        run_manager: RunManager::new(),
    };

    let app = Router::new()
        .route("/api/config", get(get_config).post(set_config))
        .route("/api/workshop/resolve", axum::routing::post(resolve_workshop))
        .route("/api/settings", get(get_settings_api).post(save_settings_api))
        .route("/api/steamcmd/update", axum::routing::post(steamcmd_update))
        .route("/api/run/status", get(run_status))
        .route("/api/run/start", axum::routing::post(run_start))
        .route("/api/run/stop", axum::routing::post(run_stop))
        .route("/api/run/logs/tail", get(run_logs_tail))
        .route("/api/run/logs/stream", get(run_logs_stream))
        .route("/server", get(profiles_page))
        .route("/server/:profile_id", get(profile_detail))
        .route("/server/:profile_id/activate", axum::routing::post(activate_profile))
        .route("/server/:profile_id/edit", get(edit_profile_page).post(save_profile_edit))
        .route("/server/:profile_id/delete", axum::routing::post(delete_profile_action))
        .route("/server/new", get(new_profile_page))
        .route("/server/new/resolve", axum::routing::post(new_profile_resolve))
        .route("/server/new/create", axum::routing::post(new_profile_create))
        .route(
            "/server/:profile_id/workshop",
            get(profile_workshop_page),
        )
        .route(
            "/server/:profile_id/workshop/resolve",
            axum::routing::post(profile_workshop_resolve),
        )
        .route(
            "/server/:profile_id/workshop/save",
            axum::routing::post(profile_workshop_save),
        )
        .route(
            "/server/:profile_id/config-preview",
            get(config_preview_page).post(config_preview_partial),
        )
        .route(
            "/server/:profile_id/config-write",
            axum::routing::post(write_config),
        )
        .route(
            "/server/:profile_id/config-regenerate",
            axum::routing::post(regenerate_config),
        )
        .route("/packages", get(packages_page))
        .route("/packages/mods/add", axum::routing::post(add_mod))
        .route("/packages/mods/:mod_id/edit", axum::routing::post(edit_mod))
        .route("/packages/mods/:mod_id/delete", axum::routing::post(delete_mod))
        .route("/packages/packs/add", axum::routing::post(add_package))
        .route("/packages/packs/:package_id", get(package_edit_page))
        .route("/packages/packs/:package_id/edit", axum::routing::post(edit_package))
        .route("/packages/packs/:package_id/delete", axum::routing::post(delete_package))
        .route("/run-logs", get(run_logs_page))
        .route("/settings", get(settings_page).post(settings_save))
        .route("/settings/defaults", axum::routing::post(settings_defaults_save))
        .route("/partials/header-status", get(header_status_partial))
        .route("/partials/server-status-card", get(server_status_card).post(server_status_action))
        .route("/health", get(health))
        .route("/", get(dashboard_page))
        .nest_service("/web", ServeDir::new(web_dir))
        .with_state(state);
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

fn wants_html(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.contains("text/html"))
        .unwrap_or(false)
}

fn health_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>ARSSM Health</title>
    <style>
      body { font-family: system-ui, sans-serif; margin: 2rem; }
      label { display: block; margin-bottom: 0.5rem; }
      input { width: 100%; max-width: 720px; padding: 0.5rem; }
      button { margin-top: 0.75rem; padding: 0.5rem 1rem; }
      pre { background: #f4f4f4; padding: 1rem; white-space: pre-wrap; }
    </style>
  </head>
  <body>
    <h1>ARSSM</h1>
    <p>Status: ok</p>
    <label for="workshop-url">Workshop URL</label>
    <input id="workshop-url" type="text" value="https://reforger.armaplatform.com/workshop/595F2BF2F44836FB-RHS-StatusQuo">
    <button id="resolve">Resolve</button>
    <h2>Result</h2>
    <pre id="output">Waiting for input.</pre>
    <script>
      const button = document.getElementById('resolve');
      const output = document.getElementById('output');
      button.addEventListener('click', async () => {
        output.textContent = 'Resolving...';
        const url = document.getElementById('workshop-url').value;
        try {
          const response = await fetch('/api/workshop/resolve', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ url, max_depth: 5 })
          });
          const data = await response.json();
          output.textContent = JSON.stringify(data, null, 2);
        } catch (error) {
          output.textContent = 'Error: ' + error;
        }
      });
    </script>
  </body>
</html>
"#
}

async fn profiles_page(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profiles_page(&profiles, settings.active_profile_id.as_deref(), None)))
}


async fn profile_detail(
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

async fn edit_profile_page(
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    Ok(Html(render_profile_edit(&profile, None)))
}

#[derive(Deserialize)]
struct EditProfileForm {
    display_name: String,
    workshop_url: String,
}

async fn save_profile_edit(
    Path(profile_id): Path<String>,
    Form(form): Form<EditProfileForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    if form.display_name.trim().is_empty() || form.workshop_url.trim().is_empty() {
        return Ok(Html(render_profile_edit(
            &profile,
            Some("Display name and workshop URL are required."),
        )));
    }

    profile.display_name = form.display_name.trim().to_string();
    profile.workshop_url = form.workshop_url.trim().to_string();
    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profile_edit(&profile, Some("Profile updated."))))
}

async fn delete_profile_action(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    delete_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let mut settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    if settings.active_profile_id.as_deref() == Some(&profile_id) {
        settings.active_profile_id = None;
        save_settings(&state.settings_path, &settings)
            .await
            .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    }

    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profiles_page(
        &profiles,
        settings.active_profile_id.as_deref(),
        Some("Profile deleted."),
    )))
}

async fn new_profile_page() -> Result<Html<String>, (StatusCode, String)> {
    Ok(Html(render_new_profile_wizard(None)))
}

#[derive(Deserialize)]
struct NewProfileResolveForm {
    workshop_url: String,
}

async fn new_profile_resolve(
    State(state): State<AppState>,
    Form(form): Form<NewProfileResolveForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let url = form.workshop_url.trim();
    if url.is_empty() {
        return Ok(Html(render_new_profile_resolve(None, Some("Workshop URL is required."))));
    }

    let result = state
        .workshop_resolver
        .resolve(url, 5)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;

    Ok(Html(render_new_profile_resolve(Some(&result), None)))
}

#[derive(Deserialize)]
struct NewProfileCreateForm {
    display_name: String,
    workshop_url: String,
    root_mod_id: Option<String>,
    dependency_mod_ids: Option<String>,
    selected_scenario_id_path: Option<String>,
    optional_mod_ids: Option<String>,
}

async fn new_profile_create(
    Form(form): Form<NewProfileCreateForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    if form.display_name.trim().is_empty() || form.workshop_url.trim().is_empty() {
        return Ok(Html(render_new_profile_wizard(Some(
            "Display name and workshop URL are required.",
        ))));
    }

    let selected_scenario = form
        .selected_scenario_id_path
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_string();
    if selected_scenario.is_empty() {
        return Ok(Html(render_new_profile_wizard(Some(
            "Scenario selection is required.",
        ))));
    }

    let root_mod_id = form.root_mod_id.as_deref().unwrap_or("").trim().to_string();
    if root_mod_id.is_empty() {
        return Ok(Html(render_new_profile_wizard(Some(
            "Workshop must be resolved before creating the profile.",
        ))));
    }

    let profile = ServerProfile {
        profile_id: new_profile_id(),
        display_name: form.display_name.trim().to_string(),
        workshop_url: form.workshop_url.trim().to_string(),
        root_mod_id: Some(root_mod_id),
        selected_scenario_id_path: Some(selected_scenario),
        dependency_mod_ids: parse_mod_ids(form.dependency_mod_ids.as_deref().unwrap_or("")),
        optional_mod_ids: parse_mod_ids(form.optional_mod_ids.as_deref().unwrap_or("")),
        load_session_save: false,
        generated_config_path: None,
        last_resolved_at: None,
        last_resolve_hash: None,
    };

    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profile_detail(&profile, None)))
}

async fn activate_profile(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let _profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    let mut settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    settings.active_profile_id = Some(profile_id);
    save_settings(&state.settings_path, &settings)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profiles_page(
        &profiles,
        settings.active_profile_id.as_deref(),
        Some("Active profile updated."),
    )))
}

async fn packages_page() -> Result<Html<String>, (StatusCode, String)> {
    let mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let content = render_packages_page(&mods, &packages, None);
    Ok(Html(render_layout(
        "ARSSM Pakete",
        "packages",
        vec![breadcrumb("Pakete / Mods", None)],
        &content,
    )))
}

#[derive(Deserialize)]
struct ModForm {
    mod_id: String,
    name: String,
}

async fn add_mod(
    Form(form): Form<ModForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if form.mod_id.trim().is_empty() || form.name.trim().is_empty() {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Mod ID and name are required."),
        )));
    }

    let mod_id = parse_mod_id_input(&form.mod_id)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid mod ID".to_string()))?;
    if mods.iter().any(|entry| entry.mod_id == mod_id) {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Mod ID already exists."),
        )));
    }

    mods.push(backend::models::ModEntry {
        mod_id,
        name: form.name.trim().to_string(),
    });
    save_mods(&mods)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page(&mods, &packages, Some("Mod added."))))
}

async fn edit_mod(
    Path(mod_id): Path<String>,
    Form(form): Form<ModForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if form.name.trim().is_empty() {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Mod name is required."),
        )));
    }

    let updated = mods.iter_mut().any(|entry| {
        if entry.mod_id == mod_id {
            entry.name = form.name.trim().to_string();
            true
        } else {
            false
        }
    });

    if !updated {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Mod not found."),
        )));
    }

    save_mods(&mods)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page(&mods, &packages, Some("Mod updated."))))
}

async fn delete_mod(
    Path(mod_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if packages.iter().any(|package| package.mod_ids.iter().any(|id| id == &mod_id)) {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Mod is used in a package and cannot be deleted."),
        )));
    }

    mods.retain(|entry| entry.mod_id != mod_id);
    save_mods(&mods)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page(&mods, &packages, Some("Mod deleted."))))
}

#[derive(Deserialize)]
struct PackageForm {
    name: String,
    mod_ids: Option<Vec<String>>,
}

async fn add_package(
    Form(form): Form<PackageForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let mut packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if form.name.trim().is_empty() {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Package name is required."),
        )));
    }

    let package = backend::models::ModPackage {
        package_id: new_package_id(),
        name: form.name.trim().to_string(),
        mod_ids: form.mod_ids.unwrap_or_default(),
    };
    packages.push(package);
    save_packages(&packages)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page(
        &mods,
        &packages,
        Some("Package created."),
    )))
}

async fn edit_package(
    Path(package_id): Path<String>,
    Form(form): Form<PackageForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let mut packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if form.name.trim().is_empty() {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Package name is required."),
        )));
    }

    let updated = packages.iter_mut().any(|entry| {
        if entry.package_id == package_id {
            entry.name = form.name.trim().to_string();
            entry.mod_ids = form.mod_ids.clone().unwrap_or_default();
            true
        } else {
            false
        }
    });

    if !updated {
        return Ok(Html(render_packages_page(
            &mods,
            &packages,
            Some("Package not found."),
        )));
    }

    save_packages(&packages)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page(
        &mods,
        &packages,
        Some("Package updated."),
    )))
}

async fn delete_package(
    Path(package_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let mut packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    packages.retain(|entry| entry.package_id != package_id);
    save_packages(&packages)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page(
        &mods,
        &packages,
        Some("Package deleted."),
    )))
}

async fn package_edit_page(
    Path(package_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let package = packages
        .iter()
        .find(|entry| entry.package_id == package_id)
        .cloned()
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Package not found".to_string()))?;
    Ok(Html(render_package_edit_page(&package, &mods)))
}

async fn profile_workshop_page(
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    Ok(Html(render_workshop_page(&profile, None, None)))
}

async fn profile_workshop_resolve(
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

#[derive(Deserialize)]
struct WorkshopSaveForm {
    selected_scenario_id_path: String,
    optional_mod_ids: String,
}

async fn profile_workshop_save(
    State(state): State<AppState>,
    Path(profile_id): Path<String>,
    Form(form): Form<WorkshopSaveForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    let scenario = form.selected_scenario_id_path.trim().to_string();
    if scenario.is_empty() {
        return Ok(Html(render_workshop_page(
            &profile,
            None,
            Some("Scenario selection is required."),
        )));
    }

    profile.selected_scenario_id_path = Some(scenario);
    profile.optional_mod_ids = parse_mod_ids(&form.optional_mod_ids);
    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let resolved = resolve_and_update_profile(&state, &mut profile)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;

    Ok(Html(render_workshop_page(
        &profile,
        Some(&resolved),
        Some("Workshop selections saved."),
    )))
}

async fn config_preview_page(
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    let preview = match generate_config_for_profile(&profile) {
        Ok(value) => serde_json::to_string_pretty(&value)
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?,
        Err(err) => err,
    };

    Ok(Html(render_config_preview(&profile, &preview, None)))
}

async fn config_preview_partial(
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

    let preview = match generate_config_for_profile(&profile) {
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

async fn write_config(
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

    let config = generate_config_for_profile(&profile)
        .map_err(|message| (StatusCode::BAD_REQUEST, message))?;
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?;

    let path = generated_config_path(&profile.profile_id);
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

async fn regenerate_config(
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
        return Ok(Html(render_config_preview(&profile, &preview, Some("Scenario selection invalid."))));
    } else if resolve_result.errors.is_empty() {
        Some("Config regenerated.")
    } else {
        Some("Config regenerated with resolve warnings.")
    };

    let preview = match generate_config_for_profile(&profile) {
        Ok(value) => serde_json::to_string_pretty(&value)
            .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?,
        Err(err) => err,
    };

    Ok(Html(render_config_preview(&profile, &preview, notice)))
}

#[derive(Deserialize)]
struct SettingsQuery {
    tab: Option<String>,
}

async fn settings_page(
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

async fn run_logs_page() -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_run_logs_page(&profiles)))
}

async fn header_status_partial(
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

    let context = context! {
        datetime => datetime,
        run_status => run_status,
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

async fn server_status_card(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let status = state.run_manager.status().await;
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let active_name = active_profile_name(settings.active_profile_id.as_deref()).await;
    Ok(Html(render_server_status_card(&status, active_name.as_deref(), None)))
}

#[derive(Deserialize)]
struct ServerActionForm {
    action: String,
}

async fn server_status_action(
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
                if let Err(err) = start_profile(&state, &settings, &profile_id).await {
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
                if let Err(err) = start_profile(&state, &settings, &profile_id).await {
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
    let active_name = active_profile_name(active_id.as_deref()).await;
    Ok(Html(render_server_status_card(&status, active_name.as_deref(), message.as_deref())))
}

async fn dashboard_page(
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

    let content = format!(
        r#"<h1 class="h3 mb-3">Dashboard</h1>
        <div class="row g-3">
          <div class="col-lg-6">
            <div id="server-status-card" hx-get="/partials/server-status-card" hx-trigger="load, every 5s" hx-swap="outerHTML"></div>
          </div>
          <div class="col-md-6 col-lg-3">
            <div class="card card-body">
              <h2 class="h6 text-uppercase text-muted">Profile</h2>
              <p class="display-6 mb-0">{profile_count}</p>
              <p class="small text-muted mb-0">Settings: {settings_status}</p>
            </div>
          </div>
          <div class="col-md-6 col-lg-3">
            <div class="card card-body">
              <h2 class="h6 text-uppercase text-muted">Pakete</h2>
              <p class="display-6 mb-0">{package_count}</p>
              <p class="small text-muted mb-0">Optional Mods verfügbar</p>
            </div>
          </div>
        </div>"#,
        profile_count = profiles.len(),
        settings_status = settings_status,
        package_count = packages.len(),
    );

    Ok(Html(render_layout(
        "ARSSM Dashboard",
        "dashboard",
        vec![breadcrumb("Dashboard", None)],
        &content,
    )))
}

#[derive(Deserialize)]
struct SettingsForm {
    steamcmd_dir: String,
    reforger_server_exe: String,
    reforger_server_work_dir: String,
    profile_dir_base: String,
    server_path: String,
    workshop_path: String,
    mod_path: String,
}

async fn settings_save(
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
        server_path: form.server_path,
        workshop_path: form.workshop_path,
        mod_path: form.mod_path,
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

async fn settings_defaults_save(
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

async fn get_settings_api(
    State(state): State<AppState>,
) -> Result<Json<AppSettings>, (StatusCode, String)> {
    load_settings(&state.settings_path)
        .await
        .map(Json)
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))
}

async fn save_settings_api(
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
struct SteamcmdUpdateResponse {
    status: String,
    message: String,
}

async fn steamcmd_update() -> Json<SteamcmdUpdateResponse> {
    Json(SteamcmdUpdateResponse {
        status: "placeholder".to_string(),
        message: "SteamCMD update is not implemented yet.".to_string(),
    })
}

fn render_settings_page(settings: &AppSettings, tab: Option<&str>, message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();
    let active_tab = tab.unwrap_or("paths");
    let tabs = format!(
        r#"<ul class="nav nav-tabs mb-3">
          <li class="nav-item"><a class="nav-link {paths_active}" href="/settings?tab=paths">Pfade</a></li>
          <li class="nav-item"><a class="nav-link {defaults_active}" href="/settings?tab=defaults">server.json Defaults</a></li>
        </ul>"#,
        paths_active = if active_tab == "paths" { "active" } else { "" },
        defaults_active = if active_tab == "defaults" { "active" } else { "" },
    );

    let paths_content = format!(
        r#"<form method="post" action="/settings">
          <h2 class="h5">Pfade</h2>
          <div class="mb-3">
            <label class="form-label" for="server_path">Server-Pfad</label>
            <input class="form-control" id="server_path" name="server_path" value="{server_path}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="workshop_path">Workshop-Pfad</label>
            <input class="form-control" id="workshop_path" name="workshop_path" value="{workshop_path}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="mod_path">Mod-Pfad</label>
            <input class="form-control" id="mod_path" name="mod_path" value="{mod_path}">
          </div>
          <hr>
          <h2 class="h6 text-uppercase text-muted">Runtime Paths</h2>
          <div class="mb-3">
            <label class="form-label" for="steamcmd_dir">SteamCMD directory</label>
            <input class="form-control" id="steamcmd_dir" name="steamcmd_dir" value="{steamcmd_dir}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="reforger_server_exe">Reforger server executable</label>
            <input class="form-control" id="reforger_server_exe" name="reforger_server_exe" value="{reforger_server_exe}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="reforger_server_work_dir">Reforger server work dir</label>
            <input class="form-control" id="reforger_server_work_dir" name="reforger_server_work_dir" value="{reforger_server_work_dir}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="profile_dir_base">Profile base directory</label>
            <input class="form-control" id="profile_dir_base" name="profile_dir_base" value="{profile_dir_base}">
          </div>
          <button class="btn btn-primary" type="submit">Save</button>
        </form>
        <hr>
        <h2 class="h5">SteamCMD Update</h2>
        <p class="text-muted">Placeholder action for MVP2.</p>
        <button class="btn btn-outline-primary" id="steamcmd-update">Run update</button>
        <p class="mt-2" id="steamcmd-status"></p>
        <script>
          document.getElementById('steamcmd-update').addEventListener('click', async () => {{
            const status = document.getElementById('steamcmd-status');
            status.textContent = 'Running...';
            const response = await fetch('/api/steamcmd/update', {{ method: 'POST' }});
            const data = await response.json();
            status.textContent = data.message;
          }});
        </script>"#,
        server_path = html_escape::encode_text(&settings.server_path),
        workshop_path = html_escape::encode_text(&settings.workshop_path),
        mod_path = html_escape::encode_text(&settings.mod_path),
        steamcmd_dir = html_escape::encode_text(&settings.steamcmd_dir),
        reforger_server_exe = html_escape::encode_text(&settings.reforger_server_exe),
        reforger_server_work_dir = html_escape::encode_text(&settings.reforger_server_work_dir),
        profile_dir_base = html_escape::encode_text(&settings.profile_dir_base),
    );

    let defaults_content = render_defaults_form(settings);

    let content = format!(
        r#"<h1 class="h3 mb-3">Settings</h1>
        {notice}
        {tabs}
        {tab_content}"#,
        notice = notice,
        tabs = tabs,
        tab_content = if active_tab == "defaults" {
            defaults_content
        } else {
            paths_content
        },
    );

    render_layout(
        "ARSSM Settings",
        "settings",
        vec![breadcrumb("Settings", None)],
        &content,
    )
}

#[derive(Debug)]
struct DefaultField {
    path: String,
    kind: String,
    value: String,
}

fn render_defaults_form(settings: &AppSettings) -> String {
    let fields = flatten_defaults(&settings.server_json_defaults);
    let mut disabled_keys = Vec::new();
    let mut rows = String::new();
    for field in fields {
        let enabled = settings
            .server_json_enabled
            .get(&field.path)
            .copied()
            .unwrap_or(true);
        if !enabled {
            disabled_keys.push(field.path.clone());
        }
        let checked = if enabled { "checked" } else { "" };
        rows.push_str(&format!(
            r#"<tr>
              <td><input type="checkbox" name="default_enabled.{path}" {checked}></td>
              <td><code>{path}</code></td>
              <td>
                <input type="hidden" name="default_type.{path}" value="{kind}">
                <input class="form-control form-control-sm" name="default_value.{path}" value="{value}">
              </td>
            </tr>"#,
            path = html_escape::encode_text(&field.path),
            kind = html_escape::encode_text(&field.kind),
            value = html_escape::encode_text(&field.value),
            checked = checked,
        ));
    }

    let disabled_summary = if disabled_keys.is_empty() {
        "<p class=\"text-muted\">Alle Defaults aktiv.</p>".to_string()
    } else {
        let items = disabled_keys
            .iter()
            .map(|key| format!("<span class=\"badge text-bg-secondary me-1\">{}</span>", html_escape::encode_text(key)))
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "<p class=\"text-muted\">Deaktiviert: {} Optionen</p><div class=\"mb-2\">{}</div>",
            disabled_keys.len(),
            items
        )
    };

    format!(
        r#"<form method="post" action="/settings/defaults">
          <h2 class="h5">server.json Defaults</h2>
          <p class="text-muted">Aktive Optionen werden bei neuen Profilen genutzt.</p>
          {disabled_summary}
          <div class="table-responsive">
            <table class="table table-sm align-middle">
              <thead>
                <tr>
                  <th>Active</th>
                  <th>Option</th>
                  <th>Value</th>
                </tr>
              </thead>
              <tbody>
                {rows}
              </tbody>
            </table>
          </div>
          <button class="btn btn-primary" type="submit">Save defaults</button>
        </form>"#,
        rows = rows,
        disabled_summary = disabled_summary,
    )
}

fn render_profiles_page(
    profiles: &[ServerProfile],
    active_profile_id: Option<&str>,
    message: Option<&str>,
) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();

    let mut rows = String::new();
    for profile in profiles {
        let is_active = active_profile_id
            .map(|value| value == profile.profile_id)
            .unwrap_or(false);
        let active_badge = if is_active {
            "<span class=\"badge text-bg-success ms-2\">active</span>"
        } else {
            ""
        };
        rows.push_str(&format!(
            r#"<tr>
              <td><a href="/server/{id}">{name}</a> {active_badge}</td>
              <td>{url}</td>
              <td>
                <form method="post" action="/server/{id}/activate">
                  <button class="btn btn-sm btn-outline-secondary" type="submit">Set active</button>
                </form>
              </td>
            </tr>"#,
            id = html_escape::encode_text(&profile.profile_id),
            name = html_escape::encode_text(&profile.display_name),
            url = html_escape::encode_text(&profile.workshop_url),
            active_badge = active_badge,
        ));
    }

    if rows.is_empty() {
        rows.push_str("<tr><td colspan=\"3\">No profiles yet.</td></tr>");
    }

    let content = format!(
        r#"<h1 class="h3 mb-3">Server / Profile</h1>
        {notice}
        <a class="btn btn-primary mb-3" href="/server/new">Neues Profil</a>
        <table class="table table-striped">
          <thead>
            <tr>
              <th>Profile</th>
              <th>Workshop URL</th>
              <th>Active</th>
            </tr>
          </thead>
          <tbody>
            {rows}
          </tbody>
        </table>"#,
        notice = notice,
        rows = rows,
    );

    render_layout(
        "ARSSM Server / Profile",
        "server",
        vec![breadcrumb("Server / Profile", None)],
        &content,
    )
}

fn render_profile_detail(profile: &ServerProfile, active_profile_id: Option<&str>) -> String {
    let is_active = active_profile_id
        .map(|value| value == profile.profile_id)
        .unwrap_or(false);
    let active_badge = if is_active {
        "<span class=\"badge text-bg-success ms-2\">active</span>"
    } else {
        ""
    };
    let content = format!(
        r#"<h1 class="h3 mb-3">Profile: {name}</h1>
        <dl class="row">
          <dt class="col-sm-3">Profile ID</dt>
          <dd class="col-sm-9">{id}</dd>
          <dt class="col-sm-3">Workshop URL</dt>
          <dd class="col-sm-9">{url}</dd>
          <dt class="col-sm-3">Selected scenario</dt>
          <dd class="col-sm-9">{scenario}</dd>
          <dt class="col-sm-3">Active</dt>
          <dd class="col-sm-9">{active_badge}</dd>
        </dl>
        <a class="btn btn-outline-primary me-2" href="/server/{id}/workshop">Workshop resolve</a>
        <a class="btn btn-primary me-2" href="/server/{id}/config-preview">Config preview</a>
        <a class="btn btn-outline-secondary me-2" href="/server/{id}/edit">Edit</a>
        <form class="d-inline" method="post" action="/server/{id}/activate">
          <button class="btn btn-outline-secondary" type="submit">Set active</button>
        </form>
        <a class="btn btn-outline-secondary ms-2" href="/server">Back to profiles</a>"#,
        name = html_escape::encode_text(&profile.display_name),
        id = html_escape::encode_text(&profile.profile_id),
        url = html_escape::encode_text(&profile.workshop_url),
        scenario = html_escape::encode_text(
            profile
                .selected_scenario_id_path
                .as_deref()
                .unwrap_or("Not selected")
        ),
        active_badge = active_badge,
    );

    render_layout(
        "ARSSM Profile",
        "server",
        vec![
            breadcrumb("Server / Profile", Some("/server".to_string())),
            breadcrumb(&profile.display_name, None),
        ],
        &content,
    )
}

fn render_profile_edit(profile: &ServerProfile, message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();
    let content = format!(
        r#"<h1 class="h3 mb-3">Edit Profile</h1>
        {notice}
        <form method="post" action="/server/{id}/edit" class="card card-body mb-4">
          <div class="mb-3">
            <label class="form-label" for="display_name">Display name</label>
            <input class="form-control" id="display_name" name="display_name" value="{name}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="workshop_url">Workshop URL</label>
            <input class="form-control" id="workshop_url" name="workshop_url" value="{url}">
          </div>
          <div class="d-flex gap-2">
            <button class="btn btn-primary" type="submit">Save</button>
            <a class="btn btn-outline-secondary" href="/server/{id}">Cancel</a>
          </div>
        </form>
        <form method="post" action="/server/{id}/delete">
          <button class="btn btn-outline-danger" type="submit">Delete profile</button>
        </form>"#,
        notice = notice,
        id = html_escape::encode_text(&profile.profile_id),
        name = html_escape::encode_text(&profile.display_name),
        url = html_escape::encode_text(&profile.workshop_url),
    );

    render_layout(
        "ARSSM Edit Profile",
        "server",
        vec![
            breadcrumb("Server / Profile", Some("/server".to_string())),
            breadcrumb(&profile.display_name, Some(format!("/server/{}", profile.profile_id))),
            breadcrumb("Edit", None),
        ],
        &content,
    )
}

fn render_new_profile_wizard(message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();
    let content = format!(
        r##"<h1 class="h3 mb-3">Neues Profil</h1>
        {notice}
        <form method="post" action="/server/new/create">
          <div class="card card-body mb-4">
            <h2 class="h5">Schritt 1: Workshop</h2>
            <div class="mb-3">
              <label class="form-label" for="display_name">Display name</label>
              <input class="form-control" id="display_name" name="display_name">
            </div>
            <div class="mb-3">
              <label class="form-label" for="workshop_url">Workshop URL</label>
              <input class="form-control" id="workshop_url" name="workshop_url">
            </div>
            <button type="button" class="btn btn-outline-secondary" hx-post="/server/new/resolve" hx-target="#wizard-resolve" hx-swap="outerHTML" hx-include="#workshop_url">Workshop laden</button>
          </div>

          <div id="wizard-resolve">
            <div class="card card-body mb-4">
              <h2 class="h5">Schritt 2: Szenario</h2>
              <p class="text-muted">Workshop zuerst laden.</p>
            </div>
            <div class="card card-body mb-4">
              <h2 class="h5">Schritt 3: Mod-Pakete</h2>
              <p class="text-muted">Keine Pakete definiert.</p>
            </div>
            <div class="card card-body mb-4">
              <h2 class="h5">Schritt 4: Konfiguration</h2>
              <p class="text-muted">Defaults werden nach dem Laden angezeigt.</p>
            </div>
          </div>

          <div class="d-flex gap-2">
            <button class="btn btn-primary" type="submit">Profil erstellen</button>
            <a class="btn btn-outline-secondary" href="/server">Abbrechen</a>
          </div>
        </form>"##,
        notice = notice,
    );

    render_layout(
        "ARSSM New Profile",
        "server",
        vec![
            breadcrumb("Server / Profile", Some("/server".to_string())),
            breadcrumb("New Profile", None),
        ],
        &content,
    )
}

fn render_new_profile_resolve(
    resolved: Option<&backend::workshop::WorkshopResolveResult>,
    message: Option<&str>,
) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-warning\">{value}</p>"))
        .unwrap_or_default();

    let mut scenario_options = String::new();
    let mut dependency_ids = String::new();
    let mut root_id = String::new();
    let mut errors = String::new();
    if let Some(result) = resolved {
        root_id = result.root_id.clone();
        dependency_ids = result.dependency_ids.join(",");
        if result.scenarios.is_empty() {
            scenario_options.push_str("<option value=\"\">No scenarios found</option>");
        } else {
            for scenario in result.scenarios.iter() {
                scenario_options.push_str(&format!(
                    r#"<option value="{value}">{value}</option>"#,
                    value = html_escape::encode_text(scenario),
                ));
            }
        }
        if result.errors.is_empty() {
            errors.push_str("<li>No errors.</li>");
        } else {
            for err in result.errors.iter() {
                errors.push_str(&format!("<li>{}</li>", html_escape::encode_text(err)));
            }
        }
    }

    format!(
        r##"<div id="wizard-resolve">
          <div class="card card-body mb-4">
            <h2 class="h5">Schritt 2: Szenario</h2>
            {notice}
            <input type="hidden" name="root_mod_id" value="{root_id}">
            <input type="hidden" name="dependency_mod_ids" value="{dependency_ids}">
            <div class="mb-3">
              <label class="form-label" for="selected_scenario_id_path">Scenario</label>
              <select class="form-select" id="selected_scenario_id_path" name="selected_scenario_id_path">
                {scenario_options}
              </select>
            </div>
          </div>
          <div class="card card-body mb-4">
            <h2 class="h5">Schritt 3: Mod-Pakete</h2>
            <p class="text-muted">Pakete-Logik folgt.</p>
            <label class="form-label" for="optional_mod_ids">Optional mods (one ID per line)</label>
            <textarea class="form-control" id="optional_mod_ids" name="optional_mod_ids" rows="4"></textarea>
          </div>
          <div class="card card-body mb-4">
            <h2 class="h5">Schritt 4: Konfiguration</h2>
            <p class="text-muted">Standardwerte basieren auf server.sample.json.</p>
          </div>
          <div class="card card-body">
            <h2 class="h6">Resolve Errors</h2>
            <ul>{errors}</ul>
          </div>
        </div>"##,
        notice = notice,
        root_id = html_escape::encode_text(&root_id),
        dependency_ids = html_escape::encode_text(&dependency_ids),
        scenario_options = scenario_options,
        errors = errors,
    )
}

fn render_workshop_page(
    profile: &ServerProfile,
    resolved: Option<&backend::workshop::WorkshopResolveResult>,
    message: Option<&str>,
) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();

    let content = format!(
        r##"<h1 class="h3 mb-3">Workshop Resolve</h1>
        {notice}
        <div class="card card-body mb-4">
          <p class="mb-1"><strong>Profile:</strong> {name}</p>
          <p class="mb-3"><strong>Workshop URL:</strong> {url}</p>
          <form method="post" action="/server/{id}/workshop/resolve" hx-post="/server/{id}/workshop/resolve" hx-target="#workshop-resolve-panel" hx-swap="outerHTML">
            <button class="btn btn-primary" type="submit">Resolve</button>
            <a class="btn btn-outline-secondary ms-2" href="/server/{id}/config-preview">Go to Config Preview</a>
          </form>
        </div>
        {panel}"##,
        notice = notice,
        name = html_escape::encode_text(&profile.display_name),
        url = html_escape::encode_text(&profile.workshop_url),
        id = html_escape::encode_text(&profile.profile_id),
        panel = render_workshop_panel(profile, resolved, None),
    );

    render_layout(
        "ARSSM Workshop Resolve",
        "server",
        vec![
            breadcrumb("Server / Profile", Some("/server".to_string())),
            breadcrumb(&profile.display_name, Some(format!("/server/{}", profile.profile_id))),
            breadcrumb("Workshop Resolve", None),
        ],
        &content,
    )
}

fn render_workshop_panel(
    profile: &ServerProfile,
    resolved: Option<&backend::workshop::WorkshopResolveResult>,
    message: Option<&str>,
) -> String {
    let (scenarios, dependency_ids, errors) = if let Some(result) = resolved {
        (result.scenarios.clone(), result.dependency_ids.clone(), result.errors.clone())
    } else {
        (Vec::new(), profile.dependency_mod_ids.clone(), Vec::new())
    };

    let mut scenario_options = String::new();
    if scenarios.is_empty() {
        scenario_options.push_str("<option value=\"\">No scenarios found</option>");
    } else {
        for scenario in scenarios.iter() {
            let selected = if profile
                .selected_scenario_id_path
                .as_deref()
                .map(|value| value == scenario)
                .unwrap_or(false)
            {
                "selected"
            } else {
                ""
            };
            scenario_options.push_str(&format!(
                r#"<option value="{value}" {selected}>{value}</option>"#,
                value = html_escape::encode_text(scenario),
                selected = selected,
            ));
        }
    }

    let invalid_selection = profile
        .selected_scenario_id_path
        .as_deref()
        .map(|selected| !scenarios.is_empty() && !scenarios.iter().any(|value| value == selected))
        .unwrap_or(false);

    let selection_badge = if invalid_selection {
        "<span class=\"badge text-bg-warning ms-2\">Selection outdated</span>"
    } else {
        ""
    };

    let optional_mods = if profile.optional_mod_ids.is_empty() {
        String::new()
    } else {
        profile.optional_mod_ids.join("\n")
    };

    let dependency_count = dependency_ids.len();
    let mut dependency_list = String::new();
    for id in dependency_ids {
        dependency_list.push_str(&format!("<li>{}</li>", html_escape::encode_text(&id)));
    }
    if dependency_list.is_empty() {
        dependency_list.push_str("<li>No dependencies resolved.</li>");
    }

    let mut error_list = String::new();
    for err in errors {
        error_list.push_str(&format!("<li>{}</li>", html_escape::encode_text(&err)));
    }
    if error_list.is_empty() {
        error_list.push_str("<li>No errors.</li>");
    }

    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();

    format!(
        r#"<div id="workshop-resolve-panel">
        {notice}
        <div class="card card-body mb-4">
          <h2 class="h5">Scenario Selection {selection_badge}</h2>
          <form method="post" action="/server/{id}/workshop/save">
            <div class="mb-3">
              <label class="form-label" for="scenario">Scenario</label>
              <select class="form-select" id="scenario" name="selected_scenario_id_path">
                {scenario_options}
              </select>
            </div>
            <div class="mb-3">
              <label class="form-label" for="optional_mod_ids">Optional mods (one ID per line)</label>
              <textarea class="form-control" id="optional_mod_ids" name="optional_mod_ids" rows="4">{optional_mods}</textarea>
            </div>
            <div class="d-flex gap-2">
              <button class="btn btn-success" type="submit">Save selection</button>
              <a class="btn btn-outline-secondary" href="/server/{id}/config-preview">Config Preview</a>
            </div>
          </form>
        </div>

        <div class="card card-body mb-4">
          <h2 class="h5">Dependencies</h2>
          <p class="text-muted">{dependency_count} dependencies resolved.</p>
          <details>
            <summary>Show dependency list</summary>
            <ul>{dependency_list}</ul>
          </details>
        </div>

        <div class="card card-body">
          <h2 class="h5">Resolve Errors</h2>
          <ul>{error_list}</ul>
        </div>
        </div>"#,
        notice = notice,
        selection_badge = selection_badge,
        id = html_escape::encode_text(&profile.profile_id),
        scenario_options = scenario_options,
        optional_mods = html_escape::encode_text(&optional_mods),
        dependency_count = dependency_count,
        dependency_list = dependency_list,
        error_list = error_list,
    )
}

fn render_config_preview(profile: &ServerProfile, preview: &str, message: Option<&str>) -> String {
    let content = format!(
        r##"<h1 class="h3 mb-3">Config Preview</h1>
        <p class="text-muted">Profile: {name}</p>
        <div id="config-preview">
          {preview_block}
        </div>
        <div class="d-flex gap-2">
          <form method="post" action="/server/{id}/config-write">
            <button class="btn btn-primary" type="submit">Write file</button>
          </form>
          <button class="btn btn-outline-secondary" hx-post="/server/{id}/config-preview" hx-target="#config-preview" hx-swap="innerHTML">Resolve & Regenerate</button>
          <form method="post" action="/server/{id}/config-regenerate">
            <button class="btn btn-outline-secondary" type="submit">Regenerate (full)</button>
          </form>
        </div>
        <div class="mt-3">
          <a class="btn btn-outline-secondary" href="/server/{id}">Back to profile</a>
        </div>"##,
        name = html_escape::encode_text(&profile.display_name),
        id = html_escape::encode_text(&profile.profile_id),
        preview_block = render_config_preview_partial(preview, message),
    );

    render_layout(
        "ARSSM Config Preview",
        "server",
        vec![
            breadcrumb("Server / Profile", Some("/server".to_string())),
            breadcrumb(&profile.display_name, Some(format!("/server/{}", profile.profile_id))),
            breadcrumb("Config Preview", None),
        ],
        &content,
    )
}

fn render_config_preview_partial(preview: &str, message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();
    format!(
        r#"{notice}<pre class="bg-light p-3 border">{preview}</pre>"#,
        notice = notice,
        preview = html_escape::encode_text(preview),
    )
}

fn render_run_logs_page(profiles: &[ServerProfile]) -> String {
    let mut options = String::new();
    for profile in profiles {
        options.push_str(&format!(
            r#"<option value="{id}">{name}</option>"#,
            id = html_escape::encode_text(&profile.profile_id),
            name = html_escape::encode_text(&profile.display_name),
        ));
    }

    if options.is_empty() {
        options.push_str("<option value=\"\">No profiles available</option>");
    }

    let content = format!(
        r#"<h1 class="h3 mb-3">Run & Logs</h1>
        <div class="card card-body mb-3">
          <div class="row g-3 align-items-end">
            <div class="col-md-6">
              <label class="form-label" for="profile-select">Profile</label>
              <select class="form-select" id="profile-select">{options}</select>
            </div>
            <div class="col-md-6">
              <div class="d-flex gap-2">
                <button class="btn btn-success" id="start-btn">Start</button>
                <button class="btn btn-danger" id="stop-btn">Stop</button>
              </div>
            </div>
          </div>
          <p class="mt-3 mb-0"><strong>Status:</strong> <span id="status-text">unknown</span></p>
        </div>
        <div class="card">
          <div class="card-header">Live Log</div>
          <div class="card-body">
            <pre class="bg-light p-3 border" id="log-output" style="height: 360px; overflow-y: auto;"></pre>
          </div>
        </div>
        <script>
          const statusText = document.getElementById('status-text');
          const logOutput = document.getElementById('log-output');
          const profileSelect = document.getElementById('profile-select');

          function appendLine(line) {{
            logOutput.textContent += line + '\\n';
            logOutput.scrollTop = logOutput.scrollHeight;
          }}

          async function refreshStatus() {{
            const response = await fetch('/api/run/status');
            const data = await response.json();
            statusText.textContent = data.running ? ('running (pid ' + data.pid + ')') : 'stopped';
          }}

          document.getElementById('start-btn').addEventListener('click', async () => {{
            const profile_id = profileSelect.value;
            const response = await fetch('/api/run/start', {{
              method: 'POST',
              headers: {{ 'Content-Type': 'application/json' }},
              body: JSON.stringify({{ profile_id }})
            }});
            if (!response.ok) {{
              const text = await response.text();
              alert(text);
            }}
            refreshStatus();
          }});

          document.getElementById('stop-btn').addEventListener('click', async () => {{
            await fetch('/api/run/stop', {{ method: 'POST' }});
            refreshStatus();
          }});

          const eventSource = new EventSource('/api/run/logs/stream');
          eventSource.onmessage = (event) => {{
            appendLine(event.data);
          }};

          refreshStatus();
        </script>"#,
        options = options,
    );

    render_layout(
        "ARSSM Run & Logs",
        "run",
        vec![breadcrumb("Run / Logs", None)],
        &content,
    )
}

fn render_packages_page(
    mods: &[backend::models::ModEntry],
    packages: &[backend::models::ModPackage],
    message: Option<&str>,
) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();

    let mut mod_rows = String::new();
    for entry in mods {
        mod_rows.push_str(&format!(
            r#"<tr>
              <td>{mod_id}</td>
              <td>{name}</td>
              <td class="d-flex gap-2">
                <form method="post" action="/packages/mods/{mod_id}/edit" class="d-flex gap-2">
                  <input type="hidden" name="mod_id" value="{mod_id}">
                  <input class="form-control form-control-sm" name="name" value="{name}">
                  <button class="btn btn-sm btn-outline-secondary" type="submit">Save</button>
                </form>
                <form method="post" action="/packages/mods/{mod_id}/delete">
                  <button class="btn btn-sm btn-outline-danger" type="submit">Delete</button>
                </form>
              </td>
            </tr>"#,
            mod_id = html_escape::encode_text(&entry.mod_id),
            name = html_escape::encode_text(&entry.name),
        ));
    }
    if mod_rows.is_empty() {
        mod_rows.push_str("<tr><td colspan=\"3\">No mods defined.</td></tr>");
    }

    let mut package_rows = String::new();
    for package in packages {
        let mod_list = if package.mod_ids.is_empty() {
            "None".to_string()
        } else {
            package.mod_ids.join(", ")
        };
        package_rows.push_str(&format!(
            r#"<tr>
              <td>{name}</td>
              <td>{mods}</td>
              <td class="d-flex gap-2">
                <a class="btn btn-sm btn-outline-secondary" href="/packages/packs/{id}">Edit</a>
                <form method="post" action="/packages/packs/{id}/delete">
                  <button class="btn btn-sm btn-outline-danger" type="submit">Delete</button>
                </form>
              </td>
            </tr>"#,
            id = html_escape::encode_text(&package.package_id),
            name = html_escape::encode_text(&package.name),
            mods = html_escape::encode_text(&mod_list),
        ));
    }
    if package_rows.is_empty() {
        package_rows.push_str("<tr><td colspan=\"3\">No packages defined.</td></tr>");
    }

    let content = format!(
        r#"<h1 class="h3 mb-3">Pakete / Mods</h1>
        {notice}
        <div class="row g-3">
          <div class="col-lg-6">
            <div class="card card-body mb-3">
              <h2 class="h6 text-uppercase text-muted">Mods</h2>
              <form method="post" action="/packages/mods/add" class="row g-2 mb-3">
                <div class="col-md-5">
                  <input class="form-control" name="mod_id" placeholder="Mod ID or URL">
                </div>
                <div class="col-md-5">
                  <input class="form-control" name="name" placeholder="Name">
                </div>
                <div class="col-md-2 d-grid">
                  <button class="btn btn-primary" type="submit">Add</button>
                </div>
              </form>
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th>Mod ID</th>
                    <th>Name</th>
                    <th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {mod_rows}
                </tbody>
              </table>
            </div>
          </div>
          <div class="col-lg-6">
            <div class="card card-body mb-3">
              <h2 class="h6 text-uppercase text-muted">Pakete</h2>
              <form method="post" action="/packages/packs/add" class="mb-3">
                <div class="mb-2">
                  <input class="form-control" name="name" placeholder="Package name">
                </div>
                <select class="form-select" name="mod_ids" multiple>
                  {package_options}
                </select>
                <button class="btn btn-primary mt-2" type="submit">Create</button>
              </form>
              <table class="table table-sm">
                <thead>
                  <tr>
                    <th>Package</th>
                    <th>Mods</th>
                    <th>Actions</th>
                  </tr>
                </thead>
                <tbody>
                  {package_rows}
                </tbody>
              </table>
            </div>
          </div>
        </div>"#,
        notice = notice,
        mod_rows = mod_rows,
        package_rows = package_rows,
        package_options = render_mod_options(mods, None),
    );

    content
}

fn render_package_edit_page(
    package: &backend::models::ModPackage,
    mods: &[backend::models::ModEntry],
) -> String {
    let content = format!(
        r#"<h1 class="h3 mb-3">Package bearbeiten</h1>
        <form method="post" action="/packages/packs/{id}/edit" class="card card-body mb-4">
          <div class="mb-3">
            <label class="form-label" for="name">Name</label>
            <input class="form-control" id="name" name="name" value="{name}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="mod_ids">Mods</label>
            <select class="form-select" id="mod_ids" name="mod_ids" multiple>
              {options}
            </select>
          </div>
          <div class="d-flex gap-2">
            <button class="btn btn-primary" type="submit">Save</button>
            <a class="btn btn-outline-secondary" href="/packages">Back</a>
          </div>
        </form>
        <form method="post" action="/packages/packs/{id}/delete">
          <button class="btn btn-outline-danger" type="submit">Delete package</button>
        </form>"#,
        id = html_escape::encode_text(&package.package_id),
        name = html_escape::encode_text(&package.name),
        options = render_mod_options(mods, Some(&package.mod_ids)),
    );

    render_layout(
        "ARSSM Package",
        "packages",
        vec![
            breadcrumb("Pakete / Mods", Some("/packages".to_string())),
            breadcrumb(&package.name, None),
        ],
        &content,
    )
}

fn render_server_status_card(
    status: &RunStatus,
    active_profile_name: Option<&str>,
    message: Option<&str>,
) -> String {
    let run_state = if status.running { "running" } else { "stopped" };
    let profile_name = active_profile_name.unwrap_or("none");
    let notice = message
        .map(|value| format!("<p class=\"text-warning mb-2\">{value}</p>"))
        .unwrap_or_default();

    format!(
        r##"<div id="server-status-card" class="card card-body">
          <h2 class="h6 text-uppercase text-muted">Server Status</h2>
          {notice}
          <p class="mb-1"><strong>Status:</strong> {run_state}</p>
          <p class="mb-3"><strong>Aktives Profil:</strong> {profile_name}</p>
          <div class="d-flex flex-wrap gap-2">
            <form method="post" action="/partials/server-status-card" hx-post="/partials/server-status-card" hx-target="#server-status-card" hx-swap="outerHTML">
              <input type="hidden" name="action" value="start">
              <button class="btn btn-sm btn-success" type="submit">Start</button>
            </form>
            <form method="post" action="/partials/server-status-card" hx-post="/partials/server-status-card" hx-target="#server-status-card" hx-swap="outerHTML">
              <input type="hidden" name="action" value="stop">
              <button class="btn btn-sm btn-danger" type="submit">Stop</button>
            </form>
            <form method="post" action="/partials/server-status-card" hx-post="/partials/server-status-card" hx-target="#server-status-card" hx-swap="outerHTML">
              <input type="hidden" name="action" value="restart">
              <button class="btn btn-sm btn-outline-secondary" type="submit">Restart</button>
            </form>
          </div>
        </div>"##,
        notice = notice,
        run_state = run_state,
        profile_name = html_escape::encode_text(profile_name),
    )
}

#[derive(Serialize)]
struct NavItem {
    label: String,
    href: String,
    key: String,
}

#[derive(Serialize)]
struct Breadcrumb {
    label: String,
    href: Option<String>,
}

fn render_layout(title: &str, active: &str, breadcrumbs: Vec<Breadcrumb>, content: &str) -> String {
    let nav_items = vec![
        NavItem { label: "Dashboard".to_string(), href: "/".to_string(), key: "dashboard".to_string() },
        NavItem { label: "Server / Profile".to_string(), href: "/server".to_string(), key: "server".to_string() },
        NavItem { label: "Pakete / Mods".to_string(), href: "/packages".to_string(), key: "packages".to_string() },
        NavItem { label: "Run / Logs".to_string(), href: "/run-logs".to_string(), key: "run".to_string() },
        NavItem { label: "Settings".to_string(), href: "/settings".to_string(), key: "settings".to_string() },
    ];

    let env = template_env();
    let context = context! {
        title => title,
        active => active,
        nav_items => nav_items,
        breadcrumbs => breadcrumbs,
        content => content,
    };

    env.get_template("layouts/base.html")
        .and_then(|template| template.render(context))
        .unwrap_or_else(|err| format!("Template error: {err}"))
}

fn new_profile_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("profile-{nanos}")
}

fn new_package_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("package-{nanos}")
}

fn generate_config_for_profile(profile: &ServerProfile) -> Result<serde_json::Value, String> {
    let scenario = profile
        .selected_scenario_id_path
        .as_deref()
        .ok_or_else(|| "selected_scenario_id_path not set".to_string())?;

    let mut mod_ids = Vec::new();
    mod_ids.extend(profile.dependency_mod_ids.clone());
    mod_ids.extend(profile.optional_mod_ids.clone());

    generate_server_config(scenario, &mod_ids, Some(&profile.display_name))
}

async fn start_profile(
    state: &AppState,
    settings: &AppSettings,
    profile_id: &str,
) -> Result<(), String> {
    let profile = load_profile(profile_id).await?;

    let config_path = profile
        .generated_config_path
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| generated_config_path(&profile.profile_id));

    if tokio::fs::metadata(&config_path).await.is_err() {
        return Err("generated config not found".to_string());
    }

    let profile_dir = PathBuf::from(&settings.profile_dir_base).join(&profile.profile_id);

    state
        .run_manager
        .start(settings, &profile, &config_path, &profile_dir)
        .await
}

fn template_env() -> &'static Environment<'static> {
    static ENV: OnceLock<Environment<'static>> = OnceLock::new();
    ENV.get_or_init(|| {
        let mut env = Environment::new();
        env.set_loader(minijinja::path_loader(templates_dir()));
        env.set_auto_escape_callback(|_| minijinja::AutoEscape::Html);
        env
    })
}

fn templates_dir() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("templates")
        .to_string_lossy()
        .to_string()
}

fn breadcrumb(label: &str, href: Option<String>) -> Breadcrumb {
    Breadcrumb {
        label: label.to_string(),
        href,
    }
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
    profile.last_resolved_at = Some(now_timestamp());
    save_profile(profile).await?;
    Ok(result)
}

fn render_mod_options(mods: &[backend::models::ModEntry], selected: Option<&[String]>) -> String {
    let mut options = String::new();
    for entry in mods {
        let is_selected = selected
            .map(|list| list.iter().any(|id| id == &entry.mod_id))
            .unwrap_or(false);
        let selected_attr = if is_selected { "selected" } else { "" };
        options.push_str(&format!(
            r#"<option value="{id}" {selected}>{name} ({id})</option>"#,
            id = html_escape::encode_text(&entry.mod_id),
            name = html_escape::encode_text(&entry.name),
            selected = selected_attr,
        ));
    }
    if options.is_empty() {
        options.push_str("<option value=\"\">No mods available</option>");
    }
    options
}

fn parse_mod_id_input(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.contains("/workshop/") {
        return backend::workshop::extract_workshop_id_from_url(trimmed);
    }
    if trimmed.len() == 16 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(trimmed.to_string());
    }
    None
}

fn parse_mod_ids(input: &str) -> Vec<String> {
    input
        .lines()
        .flat_map(|line| line.split(','))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

fn now_timestamp() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    seconds.to_string()
}

fn current_datetime() -> String {
    let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .unwrap_or_else(|_| time::format_description::parse("[year]-[month]-[day]").expect("format"));
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    now.format(&format).unwrap_or_else(|_| "n/a".to_string())
}

fn format_duration(started_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let total = now.saturating_sub(started_at);
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    format!("{hours}h {minutes}m {seconds}s")
}

fn apply_default_server_json(settings: &mut AppSettings) {
    if !settings.server_json_defaults.is_object() {
        if let Ok(value) = serde_json::from_str(backend::config_gen::baseline_config()) {
            settings.server_json_defaults = value;
        }
    }
}

fn flatten_defaults(value: &serde_json::Value) -> Vec<DefaultField> {
    let mut fields = Vec::new();
    flatten_value(value, "", &mut fields);
    fields
}

fn flatten_value(value: &serde_json::Value, prefix: &str, out: &mut Vec<DefaultField>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, val) in map {
                let path = if prefix.is_empty() {
                    key.to_string()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten_value(val, &path, out);
            }
        }
        serde_json::Value::Array(list) => {
            let value_string = serde_json::to_string(list).unwrap_or_default();
            out.push(DefaultField {
                path: prefix.to_string(),
                kind: "array".to_string(),
                value: value_string,
            });
        }
        serde_json::Value::String(text) => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "string".to_string(),
            value: text.clone(),
        }),
        serde_json::Value::Number(num) => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "number".to_string(),
            value: num.to_string(),
        }),
        serde_json::Value::Bool(value) => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "bool".to_string(),
            value: value.to_string(),
        }),
        serde_json::Value::Null => out.push(DefaultField {
            path: prefix.to_string(),
            kind: "null".to_string(),
            value: "".to_string(),
        }),
    }
}

fn parse_defaults_form(
    form: &std::collections::HashMap<String, String>,
    baseline: &serde_json::Value,
) -> Result<(serde_json::Value, std::collections::HashMap<String, bool>), String> {
    let mut updated = baseline.clone();
    let mut enabled = std::collections::HashMap::new();

    let fields = flatten_defaults(baseline);
    for field in fields {
        enabled.insert(field.path, false);
    }
    for (key, _value) in form {
        if let Some(path) = key.strip_prefix("default_enabled.") {
            enabled.insert(path.to_string(), true);
        }
    }

    for (key, value) in form {
        if let Some(path) = key.strip_prefix("default_value.") {
            let type_key = format!("default_type.{path}");
            let kind = form.get(&type_key).map(String::as_str).unwrap_or("string");
            let parsed = parse_value_by_kind(kind, value)?;
            set_json_path(&mut updated, path, parsed)?;
        }
    }

    Ok((updated, enabled))
}

fn parse_value_by_kind(kind: &str, value: &str) -> Result<serde_json::Value, String> {
    match kind {
        "string" => Ok(serde_json::Value::String(value.to_string())),
        "number" => value
            .parse::<f64>()
            .map(serde_json::Value::from)
            .map_err(|_| format!("invalid number for {value}")),
        "bool" => match value.trim() {
            "true" => Ok(serde_json::Value::Bool(true)),
            "false" => Ok(serde_json::Value::Bool(false)),
            _ => Err(format!("invalid bool for {value}")),
        },
        "array" => serde_json::from_str(value)
            .map_err(|err| format!("invalid array json: {err}")),
        "null" => Ok(serde_json::Value::Null),
        _ => Ok(serde_json::Value::String(value.to_string())),
    }
}

fn set_json_path(
    target: &mut serde_json::Value,
    path: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = target;
    for (idx, part) in parts.iter().enumerate() {
        let is_last = idx == parts.len() - 1;
        if is_last {
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), value);
                return Ok(());
            } else {
                return Err(format!("invalid path: {path}"));
            }
        } else {
            current = current
                .get_mut(*part)
                .ok_or_else(|| format!("missing path segment: {part}"))?;
        }
    }
    Ok(())
}

async fn active_profile_name(profile_id: Option<&str>) -> Option<String> {
    let profile_id = profile_id?;
    load_profile(profile_id).await.ok().map(|profile| profile.display_name)
}

#[derive(Deserialize)]
struct RunStartRequest {
    profile_id: Option<String>,
}

async fn run_status(
    State(state): State<AppState>,
) -> Result<Json<RunStatus>, (StatusCode, String)> {
    Ok(Json(state.run_manager.status().await))
}

async fn run_start(
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

async fn run_stop(
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
struct LogTailResponse {
    lines: Vec<String>,
}

async fn run_logs_tail(
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

async fn run_logs_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let receiver = state.run_manager.subscribe();
    let stream = BroadcastStream::new(receiver)
        .filter_map(|message| message.ok())
        .map(|line| Ok(Event::default().data(line)));
    Sse::new(stream)
}

fn config_path() -> PathBuf {
    std::env::var("ARSSM_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("config")
                .join("app_config.json")
        })
}

async fn get_config(
    State(state): State<AppState>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    load_config(&state.config_path)
        .await
        .map(Json)
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))
}

async fn set_config(
    State(state): State<AppState>,
    Json(config): Json<AppConfig>,
) -> Result<Json<AppConfig>, (StatusCode, String)> {
    if let Err(message) = config.validate() {
        return Err((StatusCode::BAD_REQUEST, message));
    }

    save_config(&state.config_path, &config)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Json(config))
}

async fn resolve_workshop(
    State(state): State<AppState>,
    Json(request): Json<WorkshopResolveRequest>,
) -> Result<Json<backend::workshop::WorkshopResolveResult>, (StatusCode, String)> {
    if request.url.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "url must not be empty".to_string()));
    }

    let max_depth = request.max_depth.unwrap_or(5);
    let result = state
        .workshop_resolver
        .resolve(&request.url, max_depth)
        .await
        .map_err(|message| (StatusCode::BAD_GATEWAY, message))?;

    Ok(Json(result))
}

impl AppConfig {
    fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("steamcmd_dir", &self.steamcmd_dir),
            ("reforger_server_exe", &self.reforger_server_exe),
            ("reforger_server_work_dir", &self.reforger_server_work_dir),
            ("server_config_path", &self.server_config_path),
            ("profile_dir", &self.profile_dir),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{field} must not be empty"));
            }
        }
        Ok(())
    }
}

async fn load_config(path: &PathBuf) -> Result<AppConfig, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|err| format!("failed to parse config: {err}")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(AppConfig::default()),
        Err(err) => Err(format!("failed to read config: {err}")),
    }
}

async fn save_config(path: &PathBuf, config: &AppConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create config dir: {err}"))?;
    }

    let data = serde_json::to_string_pretty(config)
        .map_err(|err| format!("failed to serialize config: {err}"))?;

    tokio::fs::write(path, data)
        .await
        .map_err(|err| format!("failed to write config: {err}"))
}
