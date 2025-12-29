pub mod config;
pub mod dashboard;
pub mod health;
pub mod packages;
pub mod profiles;
pub mod run;
pub mod settings;
pub mod workshop;

use axum::{Router, routing::get};
use backend::{runner::RunManager, storage::settings_path, workshop::{ReqwestFetcher, WorkshopResolver}};
use std::path::PathBuf;
use tower_http::services::ServeDir;
use tokio::sync::Mutex;
use std::sync::Arc;
use sysinfo::System;

#[derive(Clone)]
pub struct AppState {
    pub config_path: PathBuf,
    pub workshop_resolver: WorkshopResolver,
    pub settings_path: PathBuf,
    pub run_manager: RunManager,
    pub system: Arc<Mutex<System>>,
}

pub fn build_router(state: AppState) -> Router {
    let web_dir = web_dir();
    Router::new()
        .route("/api/config", get(config::get_config).post(config::set_config))
        .route("/api/workshop/resolve", axum::routing::post(workshop::resolve_workshop))
        .route("/api/settings", get(settings::get_settings_api).post(settings::save_settings_api))
        .route("/api/steamcmd/update", axum::routing::post(settings::steamcmd_update))
        .route("/api/run/status", get(run::run_status))
        .route("/api/run/start", axum::routing::post(run::run_start))
        .route("/api/run/stop", axum::routing::post(run::run_stop))
        .route("/api/run/logs/tail", get(run::run_logs_tail))
        .route("/api/run/logs/stream", get(run::run_logs_stream))
        .route("/server", get(profiles::profiles_page))
        .route("/server/:profile_id", get(profiles::profile_detail))
        .route("/server/:profile_id/activate", axum::routing::post(profiles::activate_profile))
        .route("/server/:profile_id/edit", get(profiles::edit_profile_page).post(profiles::save_profile_edit))
        .route("/server/:profile_id/optional-packages", axum::routing::post(profiles::update_profile_optional_packages))
        .route("/server/:profile_id/delete", axum::routing::post(profiles::delete_profile_action))
        .route("/server/:profile_id/paths", axum::routing::post(profiles::save_profile_paths))
        .route("/server/:profile_id/overrides", axum::routing::post(profiles::save_profile_overrides))
        .route("/server/new", get(profiles::new_profile_page))
        .route("/server/new/resolve", axum::routing::post(profiles::new_profile_resolve))
        .route("/server/new/create", axum::routing::post(profiles::new_profile_create))
        .route("/server/:profile_id/workshop", get(profiles::profile_workshop_page))
        .route("/server/:profile_id/workshop/resolve", axum::routing::post(profiles::profile_workshop_resolve))
        .route("/server/:profile_id/workshop/save", axum::routing::post(profiles::profile_workshop_save))
        .route("/server/:profile_id/config-preview", get(profiles::config_preview_page).post(profiles::config_preview_partial))
        .route("/server/:profile_id/config-write", axum::routing::post(profiles::write_config))
        .route("/server/:profile_id/config-regenerate", axum::routing::post(profiles::regenerate_config))
        .route("/packages", get(packages::packages_page))
        .route("/packages/mods/add", axum::routing::post(packages::add_mod))
        .route("/packages/mods/:mod_id/edit", axum::routing::post(packages::edit_mod))
        .route("/packages/mods/:mod_id/delete", axum::routing::post(packages::delete_mod))
        .route("/packages/packs/add", axum::routing::post(packages::add_package))
        .route("/packages/packs/:package_id", get(packages::package_edit_page))
        .route("/packages/packs/:package_id/selection", axum::routing::post(packages::update_package_edit_selection))
        .route("/packages/packs/:package_id/edit", axum::routing::post(packages::edit_package))
        .route("/packages/packs/:package_id/delete", axum::routing::post(packages::delete_package))
        .route("/run-logs", get(run::run_logs_page))
        .route("/settings", get(settings::settings_page).post(settings::settings_save))
        .route("/settings/defaults", axum::routing::post(settings::settings_defaults_save))
        .route("/partials/header-status", get(dashboard::header_status_partial))
        .route("/partials/server-status-card", get(dashboard::server_status_card).post(dashboard::server_status_action))
        .route("/health", get(health::health))
        .route("/", get(dashboard::dashboard_page))
        .nest_service("/web", ServeDir::new(web_dir))
        .with_state(state)
}

pub fn default_state() -> AppState {
    AppState {
        config_path: config::config_path(),
        workshop_resolver: WorkshopResolver::new(std::sync::Arc::new(ReqwestFetcher::new())),
        settings_path: settings_path(),
        run_manager: RunManager::new(),
        system: Arc::new(Mutex::new(System::new())),
    }
}

fn web_dir() -> PathBuf {
    std::env::var("ARSSM_WEB_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("web"))
}
