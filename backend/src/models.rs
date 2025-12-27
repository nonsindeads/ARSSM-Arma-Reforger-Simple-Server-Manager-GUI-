use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub profile_id: String,
    pub display_name: String,
    pub workshop_url: String,
    #[serde(default)]
    pub root_mod_id: Option<String>,
    #[serde(default)]
    pub selected_scenario_id_path: Option<String>,
    #[serde(default)]
    pub scenarios: Vec<String>,
    #[serde(default)]
    pub dependency_mod_ids: Vec<String>,
    #[serde(default)]
    pub optional_mod_ids: Vec<String>,
    #[serde(default)]
    pub load_session_save: bool,
    #[serde(default)]
    pub server_path_override: Option<String>,
    #[serde(default)]
    pub workshop_path_override: Option<String>,
    #[serde(default)]
    pub mod_path_override: Option<String>,
    #[serde(default)]
    pub server_json_overrides: serde_json::Value,
    #[serde(default)]
    pub server_json_override_enabled: std::collections::HashMap<String, bool>,
    #[serde(default)]
    pub generated_config_path: Option<String>,
    #[serde(default)]
    pub last_resolved_at: Option<String>,
    #[serde(default)]
    pub last_resolve_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModPreset {
    pub preset_id: String,
    pub name: String,
    pub mod_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModEntry {
    pub mod_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModPackage {
    pub package_id: String,
    pub name: String,
    pub mod_ids: Vec<String>,
}
