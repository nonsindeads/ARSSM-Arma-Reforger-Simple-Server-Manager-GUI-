use crate::views::layout::{breadcrumb, render_layout};
use backend::runner::RunStatus;

pub fn render_dashboard_page(profile_count: usize, package_count: usize, settings_status: &str) -> String {
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
        profile_count = profile_count,
        package_count = package_count,
        settings_status = html_escape::encode_text(settings_status),
    );

    render_layout(
        "ARSSM Dashboard",
        "dashboard",
        vec![breadcrumb("Dashboard", None)],
        &content,
    )
}

pub fn render_server_status_card(
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
              <button class="btn btn-sm btn-arssm-primary" type="submit">Start</button>
            </form>
            <form method="post" action="/partials/server-status-card" hx-post="/partials/server-status-card" hx-target="#server-status-card" hx-swap="outerHTML">
              <input type="hidden" name="action" value="stop">
              <button class="btn btn-sm btn-arssm-danger" type="submit">Stop</button>
            </form>
            <form method="post" action="/partials/server-status-card" hx-post="/partials/server-status-card" hx-target="#server-status-card" hx-swap="outerHTML">
              <input type="hidden" name="action" value="restart">
              <button class="btn btn-sm btn-arssm-secondary" type="submit">Restart</button>
            </form>
          </div>
        </div>"##,
        notice = notice,
        run_state = run_state,
        profile_name = html_escape::encode_text(profile_name),
    )
}
