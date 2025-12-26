use axum::{
    Form, Json, Router, extract::{Path, State}, http::{HeaderMap, StatusCode}, response::{Html, IntoResponse}, routing::get
};
use backend::{
    config_gen::generate_server_config,
    models::ServerProfile,
    runner::{RunManager, RunStatus},
    storage::{
        AppSettings, generated_config_path, load_profile, load_settings, list_profiles,
        save_profile, save_settings, settings_path,
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
        .route("/server", get(profiles_page).post(create_profile))
        .route("/server/:profile_id", get(profile_detail))
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
        .route("/run-logs", get(run_logs_page))
        .route("/settings", get(settings_page).post(settings_save))
        .route("/partials/header-status", get(header_status_partial))
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

async fn profiles_page() -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_profiles_page(&profiles, None)))
}

#[derive(Deserialize)]
struct CreateProfileForm {
    display_name: String,
    workshop_url: String,
}

async fn create_profile(
    Form(form): Form<CreateProfileForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let display_name = form.display_name.trim().to_string();
    let workshop_url = form.workshop_url.trim().to_string();

    if display_name.is_empty() || workshop_url.is_empty() {
        let profiles = list_profiles()
            .await
            .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
        return Ok(Html(render_profiles_page(
            &profiles,
            Some("Display name and workshop URL are required."),
        )));
    }

    let profile = ServerProfile {
        profile_id: new_profile_id(),
        display_name,
        workshop_url,
        root_mod_id: None,
        selected_scenario_id_path: None,
        dependency_mod_ids: Vec::new(),
        optional_mod_ids: Vec::new(),
        load_session_save: false,
        generated_config_path: None,
        last_resolved_at: None,
        last_resolve_hash: None,
    };

    save_profile(&profile)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    let profiles = list_profiles()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_profiles_page(
        &profiles,
        Some("Profile created."),
    )))
}

async fn profile_detail(
    Path(profile_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profile = load_profile(&profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;
    Ok(Html(render_profile_detail(&profile)))
}

async fn packages_page() -> Result<Html<String>, (StatusCode, String)> {
    let content = "<h1 class=\"h3 mb-3\">Pakete / Mods</h1><p class=\"text-muted\">Noch keine Pakete definiert.</p>".to_string();
    Ok(Html(render_layout(
        "ARSSM Pakete",
        "packages",
        vec![breadcrumb("Pakete / Mods", None)],
        &content,
    )))
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

async fn settings_page(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_settings_page(&settings, None)))
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

async fn dashboard_page(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let profiles = list_profiles()
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
          <div class="col-md-4">
            <div class="card card-body">
              <h2 class="h6">Profiles</h2>
              <p class="display-6 mb-0">{profile_count}</p>
            </div>
          </div>
          <div class="col-md-4">
            <div class="card card-body">
              <h2 class="h6">Settings</h2>
              <p class="mb-0">{settings_status}</p>
            </div>
          </div>
          <div class="col-md-4">
            <div class="card card-body">
              <h2 class="h6">Quick Links</h2>
              <a href="/server" class="d-block">Server / Profile</a>
              <a href="/workshop" class="d-block">Workshop Resolve</a>
              <a href="/run-logs" class="d-block">Run & Logs</a>
            </div>
          </div>
        </div>"#,
        profile_count = profiles.len(),
        settings_status = settings_status,
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
}

async fn settings_save(
    State(state): State<AppState>,
    Form(form): Form<SettingsForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let settings = AppSettings {
        steamcmd_dir: form.steamcmd_dir,
        reforger_server_exe: form.reforger_server_exe,
        reforger_server_work_dir: form.reforger_server_work_dir,
        profile_dir_base: form.profile_dir_base,
    };

    if let Err(message) = settings.validate() {
        return Ok(Html(render_settings_page(&settings, Some(&message))));
    }

    save_settings(&state.settings_path, &settings)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_settings_page(&settings, Some("Settings saved."))))
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

fn render_settings_page(settings: &AppSettings, message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();
    let content = format!(
        r#"<h1 class="h3 mb-3">Settings</h1>
        {notice}
        <form method="post" action="/settings">
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
        </form>"#,
        notice = notice,
        steamcmd_dir = html_escape::encode_text(&settings.steamcmd_dir),
        reforger_server_exe = html_escape::encode_text(&settings.reforger_server_exe),
        reforger_server_work_dir = html_escape::encode_text(&settings.reforger_server_work_dir),
        profile_dir_base = html_escape::encode_text(&settings.profile_dir_base),
    );

    let content = format!(
        r#"{content}
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
        content = content
    );

    render_layout(
        "ARSSM Settings",
        "settings",
        vec![breadcrumb("Settings", None)],
        &content,
    )
}

fn render_profiles_page(profiles: &[ServerProfile], message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();

    let mut rows = String::new();
    for profile in profiles {
        rows.push_str(&format!(
            r#"<tr>
              <td><a href="/server/{id}">{name}</a></td>
              <td>{url}</td>
            </tr>"#,
            id = html_escape::encode_text(&profile.profile_id),
            name = html_escape::encode_text(&profile.display_name),
            url = html_escape::encode_text(&profile.workshop_url),
        ));
    }

    if rows.is_empty() {
        rows.push_str("<tr><td colspan=\"2\">No profiles yet.</td></tr>");
    }

    let content = format!(
        r#"<h1 class="h3 mb-3">Profiles</h1>
        {notice}
        <form method="post" action="/server" class="card card-body mb-4">
          <div class="mb-3">
            <label class="form-label" for="display_name">Display name</label>
            <input class="form-control" id="display_name" name="display_name">
          </div>
          <div class="mb-3">
            <label class="form-label" for="workshop_url">Workshop URL</label>
            <input class="form-control" id="workshop_url" name="workshop_url">
          </div>
          <button class="btn btn-primary" type="submit">Create profile</button>
        </form>
        <table class="table table-striped">
          <thead>
            <tr>
              <th>Profile</th>
              <th>Workshop URL</th>
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

fn render_profile_detail(profile: &ServerProfile) -> String {
    let content = format!(
        r#"<h1 class="h3 mb-3">Profile: {name}</h1>
        <dl class="row">
          <dt class="col-sm-3">Profile ID</dt>
          <dd class="col-sm-9">{id}</dd>
          <dt class="col-sm-3">Workshop URL</dt>
          <dd class="col-sm-9">{url}</dd>
          <dt class="col-sm-3">Selected scenario</dt>
          <dd class="col-sm-9">{scenario}</dd>
        </dl>
        <a class="btn btn-outline-primary me-2" href="/server/{id}/workshop">Workshop resolve</a>
        <a class="btn btn-primary me-2" href="/server/{id}/config-preview">Config preview</a>
        <a class="btn btn-outline-secondary" href="/server">Back to profiles</a>"#,
        name = html_escape::encode_text(&profile.display_name),
        id = html_escape::encode_text(&profile.profile_id),
        url = html_escape::encode_text(&profile.workshop_url),
        scenario = html_escape::encode_text(
            profile
                .selected_scenario_id_path
                .as_deref()
                .unwrap_or("Not selected")
        ),
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

#[derive(Deserialize)]
struct RunStartRequest {
    profile_id: String,
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
    if request.profile_id.trim().is_empty() {
        return Err((StatusCode::BAD_REQUEST, "profile_id must not be empty".to_string()));
    }

    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if let Err(message) = settings.validate() {
        return Err((StatusCode::BAD_REQUEST, message));
    }

    let profile = load_profile(&request.profile_id)
        .await
        .map_err(|message| (StatusCode::NOT_FOUND, message))?;

    let config_path = profile
        .generated_config_path
        .clone()
        .map(PathBuf::from)
        .unwrap_or_else(|| generated_config_path(&profile.profile_id));

    if tokio::fs::metadata(&config_path).await.is_err() {
        return Err((StatusCode::BAD_REQUEST, "generated config not found".to_string()));
    }

    let profile_dir = PathBuf::from(&settings.profile_dir_base).join(&profile.profile_id);

    state
        .run_manager
        .start(&settings, &profile, &config_path, &profile_dir)
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
