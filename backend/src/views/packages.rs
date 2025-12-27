use crate::views::layout::{breadcrumb, render_layout};
use backend::models::{ModEntry, ModPackage};

pub fn render_packages_page(
    mods: &[ModEntry],
    packages: &[ModPackage],
    message: Option<&str>,
) -> String {
    let notice = message
        .map(|value| format!("<p class=\"text-success\">{value}</p>"))
        .unwrap_or_default();

    let mut mod_rows = String::new();
    for entry in mods {
        mod_rows.push_str(&format!(
            r#"<tr>
              <td class="arssm-text">{mod_id}</td>
              <td class="arssm-text">{name}</td>
              <td class="d-flex gap-2">
                <form method="post" action="/packages/mods/{mod_id}/edit" class="d-flex gap-2">
                  <input type="hidden" name="mod_id" value="{mod_id}">
                  <input class="form-control form-control-sm arssm-input" name="name" value="{name}">
                  <button class="btn btn-sm btn-arssm-secondary" type="submit">Save</button>
                </form>
                <form method="post" action="/packages/mods/{mod_id}/delete">
                  <button class="btn btn-sm btn-arssm-danger" type="submit">Delete</button>
                </form>
              </td>
            </tr>"#,
            mod_id = html_escape::encode_text(&entry.mod_id),
            name = html_escape::encode_text(&entry.name),
        ));
    }
    if mod_rows.is_empty() {
        mod_rows.push_str("<tr><td colspan=\"3\" class=\"arssm-text\">No mods defined.</td></tr>");
    }

    let mut package_rows = String::new();
    for package in packages {
        package_rows.push_str(&format!(
            r#"<tr>
              <td class="arssm-text">{name}</td>
              <td class="d-flex gap-2">
                <a class="btn btn-sm btn-arssm-secondary" href="/packages/packs/{id}">Edit</a>
                <form method="post" action="/packages/packs/{id}/delete">
                  <button class="btn btn-sm btn-arssm-danger" type="submit">Delete</button>
                </form>
              </td>
            </tr>"#,
            id = html_escape::encode_text(&package.package_id),
            name = html_escape::encode_text(&package.name),
        ));
    }
    if package_rows.is_empty() {
        package_rows.push_str("<tr><td colspan=\"2\" class=\"arssm-text\">No packages defined.</td></tr>");
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
                  <input class="form-control arssm-input" name="mod_id" placeholder="Mod ID or URL">
                </div>
                <div class="col-md-5">
                  <input class="form-control arssm-input" name="name" placeholder="Name">
                </div>
                <div class="col-md-2 d-grid">
                  <button class="btn btn-arssm-primary" type="submit">Add</button>
                </div>
              </form>
              <table class="table table-sm arssm-table">
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
                  <input class="form-control arssm-input" name="name" placeholder="Package name">
                </div>
                <button class="btn btn-arssm-primary mt-2" type="submit">Create</button>
              </form>
              <table class="table table-sm arssm-table">
                <thead>
                  <tr>
                    <th>Package</th>
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
    );

    content
}

pub fn render_packages_page_full(
    mods: &[ModEntry],
    packages: &[ModPackage],
    message: Option<&str>,
) -> String {
    let content = render_packages_page(mods, packages, message);
    render_layout(
        "ARSSM Pakete",
        "packages",
        vec![breadcrumb("Pakete / Mods", None)],
        &content,
    )
}

pub fn render_package_edit_page_with_selection(
    package: &ModPackage,
    mods: &[ModEntry],
    selected_mod_ids: &[String],
) -> String {
    let selected_set: std::collections::HashSet<&String> = selected_mod_ids.iter().collect();
    let mut available_rows = String::new();
    let mut selected_rows = String::new();

    for entry in mods {
        if selected_set.contains(&entry.mod_id) {
            selected_rows.push_str(&format!(
                r#"<div class="d-flex align-items-center justify-content-between gap-2">
                  <div>
                    <div class="arssm-text">{name}</div>
                    <div class="text-muted small">{id}</div>
                  </div>
                  <form method="post" action="/packages/packs/{id}/selection">
                    {hidden_ids}
                    <input type="hidden" name="action" value="remove">
                    <input type="hidden" name="mod_id" value="{mod_id}">
                    <button class="btn btn-sm btn-arssm-danger" type="submit">Remove</button>
                  </form>
                </div>"#,
                id = html_escape::encode_text(&package.package_id),
                mod_id = html_escape::encode_text(&entry.mod_id),
                name = html_escape::encode_text(&entry.name),
                hidden_ids = crate::views::helpers::render_hidden_ids("mod_ids", selected_mod_ids),
            ));
        } else {
            available_rows.push_str(&format!(
                r#"<div class="d-flex align-items-center justify-content-between gap-2">
                  <div>
                    <div class="arssm-text">{name}</div>
                    <div class="text-muted small">{id}</div>
                  </div>
                  <form method="post" action="/packages/packs/{id}/selection">
                    {hidden_ids}
                    <input type="hidden" name="action" value="add">
                    <input type="hidden" name="mod_id" value="{mod_id}">
                    <button class="btn btn-sm btn-arssm-secondary" type="submit">Add</button>
                  </form>
                </div>"#,
                id = html_escape::encode_text(&package.package_id),
                mod_id = html_escape::encode_text(&entry.mod_id),
                name = html_escape::encode_text(&entry.name),
                hidden_ids = crate::views::helpers::render_hidden_ids("mod_ids", selected_mod_ids),
            ));
        }
    }

    if available_rows.is_empty() {
        available_rows.push_str("<div class=\"text-muted\">No available mods.</div>");
    }
    if selected_rows.is_empty() {
        selected_rows.push_str("<div class=\"text-muted\">No mods selected.</div>");
    }

    let content = format!(
        r#"<h1 class="h3 mb-3">Package bearbeiten</h1>
        <div class="card card-body mb-4">
          <h2 class="h6 text-uppercase text-muted">Mods</h2>
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
        </div>
        <form method="post" action="/packages/packs/{id}/edit" class="card card-body mb-4">
          <div class="mb-3">
            <label class="form-label" for="name">Name</label>
            <input class="form-control arssm-input" id="name" name="name" value="{name}">
          </div>
          {selected_hidden}
          <div class="d-flex gap-2">
            <button class="btn btn-arssm-primary" type="submit">Save</button>
            <a class="btn btn-arssm-secondary" href="/packages">Back</a>
          </div>
        </form>
        <form method="post" action="/packages/packs/{id}/delete">
          <button class="btn btn-arssm-danger" type="submit">Delete package</button>
        </form>"#,
        id = html_escape::encode_text(&package.package_id),
        name = html_escape::encode_text(&package.name),
        selected_hidden = crate::views::helpers::render_hidden_ids("mod_ids", selected_mod_ids),
        available_rows = available_rows,
        selected_rows = selected_rows,
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
