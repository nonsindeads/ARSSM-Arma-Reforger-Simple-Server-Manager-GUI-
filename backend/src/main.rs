use axum::{
    Form, Json, Router, extract::{Path, State}, http::{HeaderMap, StatusCode}, response::{Html, IntoResponse}, routing::get
};
use backend::{
    models::ServerProfile,
    storage::{AppSettings, load_profile, load_settings, list_profiles, save_profile, save_settings, settings_path},
    workshop::{ReqwestFetcher, WorkshopResolveRequest, WorkshopResolver},
};
use serde::{Deserialize, Serialize};
use tracing::info;
use std::{io, path::PathBuf};
use tower_http::services::{ServeDir, ServeFile};

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
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .init();

    let web_dir = web_dir();
    let index_file = web_dir.join("index.html");
    let state = AppState {
        config_path: config_path(),
        workshop_resolver: WorkshopResolver::new(std::sync::Arc::new(ReqwestFetcher::new())),
        settings_path: settings_path(),
    };

    let app = Router::new()
        .route("/api/config", get(get_config).post(set_config))
        .route("/api/workshop/resolve", axum::routing::post(resolve_workshop))
        .route("/api/settings", get(get_settings_api).post(save_settings_api))
        .route("/profiles", get(profiles_page).post(create_profile))
        .route("/profiles/:profile_id", get(profile_detail))
        .route("/settings", get(settings_page).post(settings_save))
        .route("/health", get(health))
        .route_service("/", ServeFile::new(index_file))
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

async fn settings_page(
    State(state): State<AppState>,
) -> Result<Html<String>, (StatusCode, String)> {
    let settings = load_settings(&state.settings_path)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_settings_page(&settings, None)))
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

    render_layout("ARSSM Settings", "settings", &content)
}

fn render_profiles_page(profiles: &[ServerProfile], message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();

    let mut rows = String::new();
    for profile in profiles {
        rows.push_str(&format!(
            r#"<tr>
              <td><a href="/profiles/{id}">{name}</a></td>
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
        <form method="post" action="/profiles" class="card card-body mb-4">
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

    render_layout("ARSSM Profiles", "profiles", &content)
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
        <a class="btn btn-outline-secondary" href="/profiles">Back to profiles</a>"#,
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

    render_layout("ARSSM Profile", "profiles", &content)
}

fn render_layout(title: &str, active: &str, content: &str) -> String {
    let nav_items = [
        ("Dashboard", "/", "dashboard"),
        ("Profiles", "/profiles", "profiles"),
        ("Workshop Resolve", "/workshop", "workshop"),
        ("Config Preview", "/config-preview", "config"),
        ("Run & Logs", "/run-logs", "run"),
        ("Settings", "/settings", "settings"),
    ];

    let mut nav_html = String::new();
    for (label, href, key) in nav_items {
        let class = if key == active { "nav-link active" } else { "nav-link" };
        nav_html.push_str(&format!(
            r#"<li class="nav-item"><a class="{class}" href="{href}">{label}</a></li>"#
        ));
    }

    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>{title}</title>
    <link
      href="https://cdn.jsdelivr.net/npm/bootstrap@5.3.3/dist/css/bootstrap.min.css"
      rel="stylesheet"
      integrity="sha384-QWTKZyjpPEjISv5WaRU9OFeRpok6YctnYmDr5pNlyT2bRjXh0JMhjY6hW+ALEwIH"
      crossorigin="anonymous"
    >
  </head>
  <body>
    <div class="container-fluid">
      <div class="row">
        <nav class="col-12 col-md-3 col-lg-2 bg-light border-end min-vh-100 p-3">
          <h2 class="h5">ARSSM</h2>
          <ul class="nav flex-column">
            {nav_html}
          </ul>
        </nav>
        <main class="col-12 col-md-9 col-lg-10 p-4">
          {content}
        </main>
      </div>
    </div>
  </body>
</html>"#,
        title = html_escape::encode_text(title),
        nav_html = nav_html,
        content = content,
    )
}

fn new_profile_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("profile-{nanos}")
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
