use crate::services::{format_resolve_timestamp, scenario_display_name};
use crate::views::helpers::render_hidden_ids;
use crate::views::layout::{breadcrumb, render_layout};
use backend::defaults::flatten_defaults;
use backend::models::{ModPackage, ServerProfile};

pub fn render_profiles_page(
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
              <td class="arssm-text">{url}</td>
              <td>
                <form method="post" action="/server/{id}/activate">
                  <button class="btn btn-sm btn-arssm-secondary" type="submit">Set active</button>
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
        rows.push_str("<tr><td colspan=\"3\" class=\"arssm-text\">No profiles yet.</td></tr>");
    }

    let content = format!(
        r#"<h1 class="h3 mb-3">Server / Profile</h1>
        {notice}
        <a class="btn btn-arssm-primary mb-3" href="/server/new">Neues Profil</a>
        <table class="table table-striped arssm-table">
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

pub fn render_profile_detail(profile: &ServerProfile, active_profile_id: Option<&str>) -> String {
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
          <dd class="col-sm-9 arssm-text">{url}</dd>
          <dt class="col-sm-3">Selected scenario</dt>
          <dd class="col-sm-9">{scenario_name}</dd>
          <dt class="col-sm-3">Active</dt>
          <dd class="col-sm-9">{active_badge}</dd>
          <dt class="col-sm-3">Last resolved</dt>
          <dd class="col-sm-9">{last_resolved}</dd>
        </dl>
        <a class="btn btn-arssm-secondary me-2" href="/server/{id}/workshop">Workshop resolve</a>
        <a class="btn btn-arssm-primary me-2" href="/server/{id}/config-preview">Config preview</a>
        <a class="btn btn-arssm-secondary me-2" href="/server/{id}/edit">Edit</a>
        <form class="d-inline" method="post" action="/server/{id}/activate">
          <button class="btn btn-arssm-secondary" type="submit">Set active</button>
        </form>
        <a class="btn btn-arssm-secondary ms-2" href="/server">Back to profiles</a>"#,
        name = html_escape::encode_text(&profile.display_name),
        id = html_escape::encode_text(&profile.profile_id),
        url = html_escape::encode_text(&profile.workshop_url),
        scenario_name = html_escape::encode_text(
            scenario_display_name(profile.selected_scenario_id_path.as_deref())
                .unwrap_or_else(|| "Not selected".to_string())
                .as_str()
        ),
        active_badge = active_badge,
        last_resolved = html_escape::encode_text(
            &format_resolve_timestamp(profile.last_resolved_at.as_deref())
                .unwrap_or_else(|| "Not resolved yet".to_string())
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

pub fn render_profile_edit(
    profile: &ServerProfile,
    packages: &[ModPackage],
    tab: Option<&str>,
    message: Option<&str>,
) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();
    let active_tab = tab.unwrap_or("general");
    let tabs = format!(
        r#"<ul class="nav nav-tabs mb-3">
          <li class="nav-item"><a class="nav-link {general_active}" href="/server/{id}/edit?tab=general">Allgemein</a></li>
          <li class="nav-item"><a class="nav-link {paths_active}" href="/server/{id}/edit?tab=paths">Pfade</a></li>
          <li class="nav-item"><a class="nav-link {overrides_active}" href="/server/{id}/edit?tab=overrides">server.json Overrides</a></li>
        </ul>"#,
        id = html_escape::encode_text(&profile.profile_id),
        general_active = if active_tab == "general" { "active" } else { "" },
        paths_active = if active_tab == "paths" { "active" } else { "" },
        overrides_active = if active_tab == "overrides" { "active" } else { "" },
    );

    let mut scenario_options = String::new();
    if profile.scenarios.is_empty() {
        scenario_options.push_str("<option value=\"\">Resolve workshop first</option>");
    } else {
        for scenario in profile.scenarios.iter() {
            let selected = profile
                .selected_scenario_id_path
                .as_deref()
                .map(|value| value == scenario)
                .unwrap_or(false);
            let selected_attr = if selected { "selected" } else { "" };
            scenario_options.push_str(&format!(
                r#"<option value="{value}" {selected}>{value}</option>"#,
                value = html_escape::encode_text(scenario),
                selected = selected_attr,
            ));
        }
    }

    let scenario_name = scenario_display_name(profile.selected_scenario_id_path.as_deref())
        .unwrap_or_else(|| "Not selected".to_string());
    let last_resolved = format_resolve_timestamp(profile.last_resolved_at.as_deref())
        .unwrap_or_else(|| "Not resolved yet".to_string());

    let selected_set: std::collections::HashSet<&String> =
        profile.optional_package_ids.iter().collect();
    let mut available_rows = String::new();
    let mut selected_rows = String::new();

    for package in packages {
        if selected_set.contains(&package.package_id) {
            selected_rows.push_str(&format!(
                r#"<div class="d-flex align-items-center justify-content-between gap-2">
                  <div>
                    <div class="arssm-text">{name}</div>
                    <div class="text-muted small">{id}</div>
                  </div>
                  <form method="post" action="/server/{profile_id}/optional-packages">
                    {hidden_ids}
                    <input type="hidden" name="action" value="remove">
                    <input type="hidden" name="package_id" value="{id}">
                    <button class="btn btn-sm btn-arssm-danger" type="submit">Remove</button>
                  </form>
                </div>"#,
                profile_id = html_escape::encode_text(&profile.profile_id),
                id = html_escape::encode_text(&package.package_id),
                name = html_escape::encode_text(&package.name),
                hidden_ids = render_hidden_ids("optional_package_ids", &profile.optional_package_ids),
            ));
        } else {
            available_rows.push_str(&format!(
                r#"<div class="d-flex align-items-center justify-content-between gap-2">
                  <div>
                    <div class="arssm-text">{name}</div>
                    <div class="text-muted small">{id}</div>
                  </div>
                  <form method="post" action="/server/{profile_id}/optional-packages">
                    {hidden_ids}
                    <input type="hidden" name="action" value="add">
                    <input type="hidden" name="package_id" value="{id}">
                    <button class="btn btn-sm btn-arssm-secondary" type="submit">Add</button>
                  </form>
                </div>"#,
                profile_id = html_escape::encode_text(&profile.profile_id),
                id = html_escape::encode_text(&package.package_id),
                name = html_escape::encode_text(&package.name),
                hidden_ids = render_hidden_ids("optional_package_ids", &profile.optional_package_ids),
            ));
        }
    }

    if available_rows.is_empty() {
        available_rows.push_str("<div class=\"text-muted\">No available packages.</div>");
    }
    if selected_rows.is_empty() {
        selected_rows.push_str("<div class=\"text-muted\">No packages selected.</div>");
    }

    let optional_mods = if profile.optional_mod_ids.is_empty() {
        String::new()
    } else {
        profile.optional_mod_ids.join("\n")
    };

    let selection_card = format!(
        r#"<div class="card card-body mb-4">
          <h2 class="h6 text-uppercase text-muted">Optional Packages</h2>
          <div class="row g-2">
            <div class="col-md-6">
              <div class="text-muted small mb-1">Available</div>
              <div class="d-grid gap-2">{available_rows}</div>
            </div>
            <div class="col-md-6">
              <div class="text-muted small mb-1">Selected</div>
              <div class="d-grid gap-2">{selected_rows}</div>
            </div>
          </div>
        </div>"#,
        available_rows = available_rows,
        selected_rows = selected_rows,
    );

    let general_content = format!(
        r#"{selection_card}
        <form method="post" action="/server/{id}/edit" class="card card-body mb-4">
          <h2 class="h5">Allgemein</h2>
          <div class="mb-3">
            <label class="form-label" for="display_name">Display name</label>
            <input class="form-control arssm-input" id="display_name" name="display_name" value="{name}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="workshop_url">Workshop URL</label>
            <input class="form-control arssm-input" id="workshop_url" name="workshop_url" value="{url}">
          </div>
          {selected_hidden}
          <div class="mb-3">
            <label class="form-label" for="selected_scenario_id_path">Scenario</label>
            <select class="form-select arssm-input" id="selected_scenario_id_path" name="selected_scenario_id_path" {scenario_disabled}>
              {scenario_options}
            </select>
            <div class="form-text text-muted">Selected: {scenario_name}</div>
          </div>
          <div class="mb-3">
            <label class="form-label" for="optional_mod_ids">Optional mod IDs (one per line)</label>
            <textarea class="form-control arssm-input" id="optional_mod_ids" name="optional_mod_ids" rows="4">{optional_mods}</textarea>
          </div>
          <p class="text-muted mb-3">Last resolved: {last_resolved}</p>
          <div class="d-flex gap-2">
            <button class="btn btn-arssm-primary" type="submit">Save</button>
            <a class="btn btn-arssm-secondary" href="/server/{id}">Cancel</a>
          </div>
        </form>
        <form method="post" action="/server/{id}/delete">
          <button class="btn btn-arssm-danger" type="submit">Delete profile</button>
        </form>"#,
        selection_card = selection_card,
        id = html_escape::encode_text(&profile.profile_id),
        name = html_escape::encode_text(&profile.display_name),
        url = html_escape::encode_text(&profile.workshop_url),
        scenario_options = scenario_options,
        scenario_name = html_escape::encode_text(&scenario_name),
        scenario_disabled = if profile.scenarios.is_empty() { "disabled" } else { "" },
        last_resolved = html_escape::encode_text(&last_resolved),
        selected_hidden = render_hidden_ids("optional_package_ids", &profile.optional_package_ids),
        optional_mods = html_escape::encode_text(&optional_mods),
    );

    let paths_content = format!(
        r#"<form method="post" action="/server/{id}/paths" class="card card-body mb-4">
          <h2 class="h5">Pfade (Override)</h2>
          <p class="text-muted">Leer lassen, um globale Settings zu verwenden.</p>
          <div class="mb-3">
            <label class="form-label" for="steamcmd_dir_override">SteamCMD directory</label>
            <input class="form-control arssm-input" id="steamcmd_dir_override" name="steamcmd_dir_override" value="{steamcmd_dir}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="reforger_server_exe_override">Reforger server executable</label>
            <input class="form-control arssm-input" id="reforger_server_exe_override" name="reforger_server_exe_override" value="{reforger_server_exe}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="reforger_server_work_dir_override">Reforger server work dir</label>
            <input class="form-control arssm-input" id="reforger_server_work_dir_override" name="reforger_server_work_dir_override" value="{reforger_server_work_dir}">
          </div>
          <div class="mb-3">
            <label class="form-label" for="profile_dir_base_override">Profile base directory</label>
            <input class="form-control arssm-input" id="profile_dir_base_override" name="profile_dir_base_override" value="{profile_dir_base}">
          </div>
          <button class="btn btn-arssm-primary" type="submit">Save paths</button>
        </form>"#,
        id = html_escape::encode_text(&profile.profile_id),
        steamcmd_dir = html_escape::encode_text(profile.steamcmd_dir_override.as_deref().unwrap_or("")),
        reforger_server_exe = html_escape::encode_text(profile.reforger_server_exe_override.as_deref().unwrap_or("")),
        reforger_server_work_dir = html_escape::encode_text(profile.reforger_server_work_dir_override.as_deref().unwrap_or("")),
        profile_dir_base = html_escape::encode_text(profile.profile_dir_base_override.as_deref().unwrap_or("")),
    );

    let overrides_content = render_profile_overrides_form(profile);

    let content = format!(
        r#"<h1 class="h3 mb-3">Edit Profile</h1>
        {notice}
        {tabs}
        {tab_content}"#,
        notice = notice,
        tabs = tabs,
        tab_content = if active_tab == "overrides" {
            overrides_content
        } else if active_tab == "paths" {
            paths_content
        } else {
            general_content
        },
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

pub fn render_profile_overrides_form(profile: &ServerProfile) -> String {
    let overrides = if profile.server_json_overrides.is_object() {
        profile.server_json_overrides.clone()
    } else {
        serde_json::Value::Object(serde_json::Map::new())
    };

    let fields = if overrides.is_object()
        && !overrides
            .as_object()
            .map(|map| map.is_empty())
            .unwrap_or(true)
    {
        flatten_defaults(&overrides)
    } else if let Ok(value) = serde_json::from_str(backend::config_gen::baseline_config()) {
        flatten_defaults(&value)
    } else {
        Vec::new()
    };
    let mut rows = String::new();
    for field in fields {
        let enabled = profile
            .server_json_override_enabled
            .get(&field.path)
            .copied()
            .unwrap_or(false);
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
            value = html_escape::encode_double_quoted_attribute(&field.value),
            checked = checked,
        ));
    }

    if rows.is_empty() {
        rows.push_str("<tr><td colspan=\"3\">No overrides defined yet.</td></tr>");
    }

    format!(
        r#"<form method="post" action="/server/{id}/overrides">
          <h2 class="h5">server.json Overrides</h2>
          <p class="text-muted">Aktiviere Felder, um die globalen Defaults zu überschreiben.</p>
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
          <button class="btn btn-arssm-primary" type="submit">Save overrides</button>
        </form>"#,
        id = html_escape::encode_text(&profile.profile_id),
        rows = rows,
    )
}

pub fn render_new_profile_wizard(message: Option<&str>) -> String {
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
              <input class="form-control arssm-input" id="display_name" name="display_name">
            </div>
            <div class="mb-3">
              <label class="form-label" for="workshop_url">Workshop URL</label>
              <input class="form-control arssm-input" id="workshop_url" name="workshop_url">
            </div>
            <button type="button" class="btn btn-arssm-secondary" hx-post="/server/new/resolve" hx-target="#wizard-resolve" hx-swap="outerHTML" hx-include="#workshop_url">Workshop laden</button>
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
            <button class="btn btn-arssm-primary" type="submit">Profil erstellen</button>
            <a class="btn btn-arssm-secondary" href="/server">Abbrechen</a>
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

pub fn render_new_profile_resolve(
    resolved: Option<&backend::workshop::WorkshopResolveResult>,
    message: Option<&str>,
) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-warning\">{value}</p>"))
        .unwrap_or_default();

    let mut scenario_options = String::new();
    let mut dependency_ids = String::new();
    let mut root_id = String::new();
    let mut scenario_ids = String::new();
    let mut dependency_list = String::new();
    let mut dependency_count = 0usize;
    let mut errors = String::new();
    if let Some(result) = resolved {
        root_id = result.root_id.clone();
        dependency_ids = result.dependency_ids.join(",");
        scenario_ids = result.scenarios.join("\n");
        for dep_id in result.dependency_ids.iter() {
            dependency_list.push_str(&format!(
                "<li>{}</li>",
                html_escape::encode_text(dep_id)
            ));
        }
        dependency_count = result.dependency_ids.len();
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
            <input type="hidden" name="scenario_ids" value="{scenario_ids}">
            <div class="mb-3">
              <label class="form-label" for="selected_scenario_id_path">Scenario</label>
              <select class="form-select arssm-input" id="selected_scenario_id_path" name="selected_scenario_id_path">
                {scenario_options}
              </select>
            </div>
            <p class="mb-1"><strong>Root mod ID:</strong> {root_id_display}</p>
            <p class="text-muted mb-2">{dependency_count} dependencies resolved.</p>
            <details>
              <summary>Show dependency list</summary>
              <ul>{dependency_list}</ul>
            </details>
          </div>
          <div class="card card-body mb-4">
            <h2 class="h5">Schritt 3: Mod-Pakete</h2>
            <p class="text-muted">Pakete-Logik folgt.</p>
            <label class="form-label" for="optional_mod_ids">Optional mods (one ID per line)</label>
            <textarea class="form-control arssm-input" id="optional_mod_ids" name="optional_mod_ids" rows="4"></textarea>
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
        root_id_display = html_escape::encode_text(&root_id),
        dependency_ids = html_escape::encode_text(&dependency_ids),
        scenario_ids = html_escape::encode_text(&scenario_ids),
        scenario_options = scenario_options,
        dependency_count = dependency_count,
        dependency_list = if dependency_list.is_empty() { "<li>No dependencies resolved.</li>".to_string() } else { dependency_list },
        errors = errors,
    )
}

pub fn render_workshop_page(
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
          <p class="mb-3"><strong>Workshop URL:</strong> <span class="arssm-text">{url}</span></p>
          <form method="post" action="/server/{id}/workshop/resolve" hx-post="/server/{id}/workshop/resolve" hx-target="#workshop-resolve-panel" hx-swap="outerHTML">
            <button class="btn btn-arssm-primary" type="submit">Resolve</button>
            <a class="btn btn-arssm-secondary ms-2" href="/server/{id}/config-preview">Go to Config Preview</a>
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

pub fn render_workshop_panel(
    profile: &ServerProfile,
    resolved: Option<&backend::workshop::WorkshopResolveResult>,
    message: Option<&str>,
) -> String {
    let (root_id, scenarios, dependency_ids, errors) = if let Some(result) = resolved {
        (
            Some(result.root_id.clone()),
            result.scenarios.clone(),
            result.dependency_ids.clone(),
            result.errors.clone(),
        )
    } else {
        (
            profile.root_mod_id.clone(),
            profile.scenarios.clone(),
            profile.dependency_mod_ids.clone(),
            Vec::new(),
        )
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

    let dependency_count = dependency_ids.len();
    let root_display = root_id
        .as_deref()
        .unwrap_or("Not resolved yet");
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
              <select class="form-select arssm-input" id="scenario" name="selected_scenario_id_path">
                {scenario_options}
              </select>
            </div>
            <div class="d-flex gap-2">
              <button class="btn btn-arssm-primary" type="submit">Save selection</button>
              <a class="btn btn-arssm-secondary" href="/server/{id}/config-preview">Config Preview</a>
            </div>
          </form>
        </div>

        <div class="card card-body mb-4">
          <h2 class="h5">Dependencies</h2>
          <p class="mb-1"><strong>Root mod ID:</strong> <span class="arssm-text">{root_display}</span></p>
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
        dependency_count = dependency_count,
        dependency_list = dependency_list,
        root_display = html_escape::encode_text(root_display),
        error_list = error_list,
    )
}

pub fn render_config_preview(profile: &ServerProfile, preview: &str, message: Option<&str>) -> String {
    let content = format!(
        r##"<h1 class="h3 mb-3">Config Preview</h1>
        <p class="text-muted">Profile: {name}</p>
        <div id="config-preview">
          {preview_block}
        </div>
        <div class="d-flex gap-2">
          <form method="post" action="/server/{id}/config-write">
            <button class="btn btn-arssm-primary" type="submit">Write file</button>
          </form>
          <button class="btn btn-arssm-secondary" hx-post="/server/{id}/config-preview" hx-target="#config-preview" hx-swap="innerHTML">Resolve & Regenerate</button>
          <form method="post" action="/server/{id}/config-regenerate">
            <button class="btn btn-arssm-secondary" type="submit">Regenerate (full)</button>
          </form>
        </div>
        <div class="mt-3">
          <a class="btn btn-arssm-secondary" href="/server/{id}">Back to profile</a>
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

pub fn render_config_preview_partial(preview: &str, message: Option<&str>) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();
    format!(
        r#"{notice}<pre class="arssm-log p-3">{preview}</pre>"#,
        notice = notice,
        preview = html_escape::encode_text(preview),
    )
}
