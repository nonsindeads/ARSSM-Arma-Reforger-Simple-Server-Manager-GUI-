use crate::forms::{ModForm, PackageCreateForm, PackageForm, PackageSelectionForm};
use crate::services::{parse_mod_id_input, update_list_selection};
use crate::views::packages::{render_package_edit_page_with_selection, render_packages_page_full};
use axum::{Form, extract::Path, http::StatusCode, response::Html};
use backend::storage::{load_mods, load_packages, save_mods, save_packages};

pub async fn packages_page() -> Result<Html<String>, (StatusCode, String)> {
    let mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    Ok(Html(render_packages_page_full(&mods, &packages, None)))
}

pub async fn add_mod(
    Form(form): Form<ModForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if form.mod_id.trim().is_empty() || form.name.trim().is_empty() {
        return Ok(Html(render_packages_page_full(
            &mods,
            &packages,
            Some("Mod ID and name are required."),
        )));
    }

    let mod_id = parse_mod_id_input(&form.mod_id)
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Invalid mod ID".to_string()))?;
    if mods.iter().any(|entry| entry.mod_id == mod_id) {
        return Ok(Html(render_packages_page_full(
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

    Ok(Html(render_packages_page_full(
        &mods,
        &packages,
        Some("Mod added."),
    )))
}

pub async fn edit_mod(
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
        return Ok(Html(render_packages_page_full(
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
        return Ok(Html(render_packages_page_full(
            &mods,
            &packages,
            Some("Mod not found."),
        )));
    }

    save_mods(&mods)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page_full(
        &mods,
        &packages,
        Some("Mod updated."),
    )))
}

pub async fn delete_mod(
    Path(mod_id): Path<String>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if packages.iter().any(|package| package.mod_ids.iter().any(|id| id == &mod_id)) {
        return Ok(Html(render_packages_page_full(
            &mods,
            &packages,
            Some("Mod is used in a package and cannot be deleted."),
        )));
    }

    mods.retain(|entry| entry.mod_id != mod_id);
    save_mods(&mods)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page_full(
        &mods,
        &packages,
        Some("Mod deleted."),
    )))
}

pub async fn add_package(
    Form(form): Form<PackageCreateForm>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mods = load_mods()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;
    let mut packages = load_packages()
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    if form.name.trim().is_empty() {
        return Ok(Html(render_packages_page_full(
            &mods,
            &packages,
            Some("Package name is required."),
        )));
    }

    let package = backend::models::ModPackage {
        package_id: new_package_id(),
        name: form.name.trim().to_string(),
        mod_ids: Vec::new(),
    };
    packages.push(package.clone());
    save_packages(&packages)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_package_edit_page_with_selection(
        &package,
        &mods,
        &package.mod_ids,
    )))
}

pub async fn edit_package(
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
        return Ok(Html(render_packages_page_full(
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
        return Ok(Html(render_packages_page_full(
            &mods,
            &packages,
            Some("Package not found."),
        )));
    }

    save_packages(&packages)
        .await
        .map_err(|message| (StatusCode::INTERNAL_SERVER_ERROR, message))?;

    Ok(Html(render_packages_page_full(
        &mods,
        &packages,
        Some("Package updated."),
    )))
}

pub async fn delete_package(
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

    Ok(Html(render_packages_page_full(
        &mods,
        &packages,
        Some("Package deleted."),
    )))
}

pub async fn update_package_edit_selection(
    Path(package_id): Path<String>,
    Form(form): Form<PackageSelectionForm>,
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
    let selected = update_list_selection(form.mod_ids, &form.action, &form.mod_id);
    Ok(Html(render_package_edit_page_with_selection(
        &package,
        &mods,
        &selected,
    )))
}

pub async fn package_edit_page(
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
    Ok(Html(render_package_edit_page_with_selection(
        &package,
        &mods,
        &package.mod_ids,
    )))
}

fn new_package_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("package-{nanos}")
}
