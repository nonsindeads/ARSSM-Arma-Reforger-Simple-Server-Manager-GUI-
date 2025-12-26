use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use crate::models::ServerProfile;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub steamcmd_dir: String,
    pub reforger_server_exe: String,
    pub reforger_server_work_dir: String,
    pub profile_dir_base: String,
}

impl AppSettings {
    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("steamcmd_dir", &self.steamcmd_dir),
            ("reforger_server_exe", &self.reforger_server_exe),
            ("reforger_server_work_dir", &self.reforger_server_work_dir),
            ("profile_dir_base", &self.profile_dir_base),
        ] {
            if value.trim().is_empty() {
                return Err(format!("{field} must not be empty"));
            }
        }
        Ok(())
    }
}

pub fn base_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata).join("arssm");
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return PathBuf::from(xdg).join("arssm");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".config").join("arssm");
    }
    PathBuf::from("arssm-data")
}

pub fn settings_path() -> PathBuf {
    base_dir().join("settings.json")
}

pub fn profiles_dir() -> PathBuf {
    base_dir().join("profiles")
}

pub fn profile_path(profile_id: &str) -> PathBuf {
    profiles_dir().join(format!("{profile_id}.json"))
}

pub async fn load_settings(path: &Path) -> Result<AppSettings, String> {
    match tokio::fs::read_to_string(path).await {
        Ok(contents) => serde_json::from_str(&contents)
            .map_err(|err| format!("failed to parse settings: {err}")),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(err) => Err(format!("failed to read settings: {err}")),
    }
}

pub async fn save_settings(path: &Path, settings: &AppSettings) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create settings dir: {err}"))?;
    }

    let data = serde_json::to_string_pretty(settings)
        .map_err(|err| format!("failed to serialize settings: {err}"))?;

    let tmp_path = path.with_extension("json.tmp");
    tokio::fs::write(&tmp_path, data)
        .await
        .map_err(|err| format!("failed to write temp settings: {err}"))?;

    if tokio::fs::metadata(path).await.is_ok() {
        tokio::fs::remove_file(path)
            .await
            .map_err(|err| format!("failed to remove old settings: {err}"))?;
    }

    tokio::fs::rename(&tmp_path, path)
        .await
        .map_err(|err| format!("failed to move settings into place: {err}"))
}

pub async fn list_profiles() -> Result<Vec<ServerProfile>, String> {
    let dir = profiles_dir();
    let mut profiles = Vec::new();

    let mut entries = match tokio::fs::read_dir(&dir).await {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(format!("failed to read profiles dir: {err}")),
    };

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|err| format!("failed to read profiles dir: {err}"))?
    {
        if entry
            .path()
            .extension()
            .and_then(|ext| ext.to_str())
            != Some("json")
        {
            continue;
        }

        let contents = tokio::fs::read_to_string(entry.path())
            .await
            .map_err(|err| format!("failed to read profile: {err}"))?;
        let profile = serde_json::from_str::<ServerProfile>(&contents)
            .map_err(|err| format!("failed to parse profile: {err}"))?;
        profiles.push(profile);
    }

    profiles.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    Ok(profiles)
}

pub async fn load_profile(profile_id: &str) -> Result<ServerProfile, String> {
    let path = profile_path(profile_id);
    let contents = tokio::fs::read_to_string(&path)
        .await
        .map_err(|err| format!("failed to read profile: {err}"))?;
    serde_json::from_str(&contents).map_err(|err| format!("failed to parse profile: {err}"))
}

pub async fn save_profile(profile: &ServerProfile) -> Result<(), String> {
    let path = profile_path(&profile.profile_id);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|err| format!("failed to create profiles dir: {err}"))?;
    }

    let data = serde_json::to_string_pretty(profile)
        .map_err(|err| format!("failed to serialize profile: {err}"))?;

    let tmp_path = path.with_extension("json.tmp");
    tokio::fs::write(&tmp_path, data)
        .await
        .map_err(|err| format!("failed to write temp profile: {err}"))?;

    if tokio::fs::metadata(&path).await.is_ok() {
        tokio::fs::remove_file(&path)
            .await
            .map_err(|err| format!("failed to remove old profile: {err}"))?;
    }

    tokio::fs::rename(&tmp_path, &path)
        .await
        .map_err(|err| format!("failed to move profile into place: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn base_dir_prefers_appdata() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let original = std::env::var("APPDATA").ok();
        unsafe {
            std::env::set_var("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        }

        let base = base_dir();
        assert!(base.to_string_lossy().contains("AppData"));
        assert!(base.to_string_lossy().ends_with("arssm"));

        if let Some(value) = original {
            unsafe {
                std::env::set_var("APPDATA", value);
            }
        } else {
            unsafe {
                std::env::remove_var("APPDATA");
            }
        }
    }
}
