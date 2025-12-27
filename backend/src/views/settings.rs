use crate::views::layout::{breadcrumb, render_layout};
use backend::defaults::flatten_defaults;
use backend::storage::AppSettings;

pub fn render_settings_page(settings: &AppSettings, tab: Option<&str>, message: Option<&str>) -> String {
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
            <label class="form-label" for="steamcmd_dir">SteamCMD directory</label>
            <input class="form-control arssm-input" id="steamcmd_dir" name="steamcmd_dir" value="{steamcmd_dir}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="reforger_server_exe">Reforger server executable</label>
            <input class="form-control arssm-input" id="reforger_server_exe" name="reforger_server_exe" value="{reforger_server_exe}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="reforger_server_work_dir">Reforger server work dir</label>
            <input class="form-control arssm-input" id="reforger_server_work_dir" name="reforger_server_work_dir" value="{reforger_server_work_dir}">
            <div class="form-text text-muted">Configs are written to <code>configs/&lt;profile_id&gt;/server.json</code> under this directory.</div>
          </div>
          <div class="mb-3">
            <label class="form-label" for="profile_dir_base">Profile base directory</label>
            <input class="form-control arssm-input" id="profile_dir_base" name="profile_dir_base" value="{profile_dir_base}">
            <div class="form-text text-muted">Profile runtime data is stored under <code>&lt;base&gt;/&lt;profile_id&gt;</code>.</div>
          </div>
          <button class="btn btn-arssm-primary" type="submit">Save</button>
        </form>
        <hr>
        <h2 class="h5">SteamCMD Update</h2>
        <p class="text-muted">Placeholder action for MVP2.</p>
        <button class="btn btn-arssm-secondary" id="steamcmd-update">Run update</button>
        <p class="mt-2" id="steamcmd-status"></p>
        <script>
          document.getElementById('steamcmd-update').addEventListener('click', async () => {
            const status = document.getElementById('steamcmd-status');
            status.textContent = 'Running...';
            const response = await fetch('/api/steamcmd/update', { method: 'POST' });
            const data = await response.json();
            status.textContent = data.message;
          });
        </script>"#,
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

pub fn render_defaults_form(settings: &AppSettings) -> String {
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
                <input class="form-control form-control-sm arssm-input" name="default_value.{path}" value="{value}">
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
            <table class="table table-sm align-middle arssm-table">
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
          <button class="btn btn-arssm-primary" type="submit">Save defaults</button>
        </form>"#,
        rows = rows,
        disabled_summary = disabled_summary,
    )
}
