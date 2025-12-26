use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerProfile {
    pub profile_id: String,
    pub display_name: String,
    pub workshop_url: String,
    pub root_mod_id: String,
    pub selected_scenario_id_path: String,
    pub dependency_mod_ids: Vec<String>,
    pub optional_mod_ids: Vec<String>,
    pub generated_config_path: String,
    pub last_resolved_at: Option<String>,
    pub last_resolve_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModPreset {
    pub preset_id: String,
    pub name: String,
    pub mod_ids: Vec<String>,
}
