use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppConfig {
    pub steamcmd_dir: String,
    pub reforger_server_exe: String,
    pub reforger_server_work_dir: String,
    pub server_config_path: String,
    pub profile_dir: String,
    pub load_session_save: bool,
}

#[derive(Deserialize)]
pub struct ProfileTabQuery {
    pub tab: Option<String>,
}

#[derive(Deserialize)]
pub struct SettingsQuery {
    pub tab: Option<String>,
}

#[derive(Deserialize)]
pub struct SettingsForm {
    pub steamcmd_dir: String,
    pub reforger_server_exe: String,
    pub reforger_server_work_dir: String,
    pub profile_dir_base: String,
}

#[derive(Deserialize)]
pub struct ModForm {
    pub mod_id: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct PackageForm {
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_mod_ids")]
    pub mod_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct PackageCreateForm {
    pub name: String,
}

#[derive(Deserialize)]
pub struct PackageSelectionForm {
    pub action: String,
    pub mod_id: String,
    #[serde(default, deserialize_with = "deserialize_mod_ids")]
    pub mod_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct NewProfileResolveForm {
    pub workshop_url: String,
}

#[derive(Deserialize)]
pub struct NewProfileCreateForm {
    pub display_name: String,
    pub workshop_url: String,
    pub root_mod_id: Option<String>,
    pub dependency_mod_ids: Option<String>,
    pub selected_scenario_id_path: Option<String>,
    pub scenario_ids: Option<String>,
    pub optional_mod_ids: Option<String>,
}

#[derive(Deserialize)]
pub struct EditProfileForm {
    pub display_name: String,
    pub workshop_url: String,
    pub selected_scenario_id_path: Option<String>,
    #[serde(default, deserialize_with = "deserialize_mod_ids")]
    pub optional_package_ids: Option<Vec<String>>,
    pub optional_mod_ids: Option<String>,
}

#[derive(Deserialize)]
pub struct OptionalPackagesForm {
    pub action: String,
    pub package_id: String,
    #[serde(default, deserialize_with = "deserialize_mod_ids")]
    pub optional_package_ids: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct ProfilePathsForm {
    pub steamcmd_dir_override: String,
    pub reforger_server_exe_override: String,
    pub reforger_server_work_dir_override: String,
    pub profile_dir_base_override: String,
}

#[derive(Deserialize)]
pub struct WorkshopSaveForm {
    pub selected_scenario_id_path: String,
}

#[derive(Deserialize)]
pub struct RunStartRequest {
    pub profile_id: Option<String>,
}

pub fn deserialize_mod_ids<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ModIdsVisitor;

    impl<'de> de::Visitor<'de> for ModIdsVisitor {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string or a list of strings")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let values = value
                .split(',')
                .map(|item| item.trim())
                .filter(|item| !item.is_empty())
                .map(|item| item.to_string())
                .collect::<Vec<_>>();
            Ok(Some(values))
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value)
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(item) = seq.next_element::<String>()? {
                let trimmed = item.trim();
                if !trimmed.is_empty() {
                    values.push(trimmed.to_string());
                }
            }
            Ok(Some(values))
        }
    }

    deserializer.deserialize_any(ModIdsVisitor)
}
