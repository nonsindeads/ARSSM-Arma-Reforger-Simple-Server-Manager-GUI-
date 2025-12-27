use backend::{
    config_gen::generate_server_config,
    defaults,
    models::{ModPackage, ServerProfile},
    storage::AppSettings,
    workshop,
};

pub fn generate_config_for_profile(
    profile: &ServerProfile,
    settings: &AppSettings,
    packages: &[ModPackage],
) -> Result<serde_json::Value, String> {
    let scenario = profile
        .selected_scenario_id_path
        .as_deref()
        .ok_or_else(|| "selected_scenario_id_path not set".to_string())?;

    let mut mod_ids = Vec::new();
    let root_mod_id = profile
        .root_mod_id
        .clone()
        .or_else(|| workshop::extract_workshop_id_from_url(&profile.workshop_url))
        .ok_or_else(|| "root_mod_id not set".to_string())?;
    mod_ids.push(root_mod_id);
    mod_ids.extend(profile.dependency_mod_ids.clone());
    mod_ids.extend(collect_optional_mod_ids(profile, packages));

    let mut config = generate_server_config(scenario, &mod_ids, Some(&profile.display_name))?;
    defaults::apply_default_server_json_settings(&mut config, settings);
    defaults::apply_profile_overrides(&mut config, profile)?;
    backend::config_gen::apply_game_overrides(
        &mut config,
        scenario,
        &mod_ids,
        Some(&profile.display_name),
    )?;

    Ok(config)
}

pub fn collect_optional_mod_ids(profile: &ServerProfile, packages: &[ModPackage]) -> Vec<String> {
    let mut ids = Vec::new();
    for package_id in profile.optional_package_ids.iter() {
        if let Some(package) = packages.iter().find(|entry| &entry.package_id == package_id) {
            ids.extend(package.mod_ids.clone());
        }
    }
    ids.extend(profile.optional_mod_ids.clone());
    ids
}

pub fn parse_mod_id_input(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.contains("/workshop/") {
        return workshop::extract_workshop_id_from_url(trimmed);
    }
    if trimmed.len() == 16 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(trimmed.to_string());
    }
    None
}

pub fn parse_mod_ids(input: &str) -> Vec<String> {
    input
        .lines()
        .flat_map(|line| line.split(','))
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

pub fn parse_scenario_ids(input: &str) -> Vec<String> {
    input
        .lines()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .collect()
}

pub fn normalize_optional_path(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn effective_value<'a>(override_value: &'a Option<String>, fallback: &'a str) -> &'a str {
    override_value
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(fallback)
}

pub fn update_list_selection(current: Option<Vec<String>>, action: &str, item_id: &str) -> Vec<String> {
    let mut selected = current.unwrap_or_default();
    match action {
        "add" => {
            if !selected.iter().any(|id| id == item_id) {
                selected.push(item_id.to_string());
            }
        }
        "remove" => {
            selected.retain(|id| id != item_id);
        }
        _ => {}
    }
    selected
}

pub fn scenario_display_name(path: Option<&str>) -> Option<String> {
    let path = path?;
    let marker = "Missions/";
    let start = path.find(marker).map(|idx| idx + marker.len())?;
    let name_with_ext = &path[start..];
    let name = name_with_ext.strip_suffix(".conf").unwrap_or(name_with_ext);
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

pub fn format_resolve_timestamp(value: Option<&str>) -> Option<String> {
    let raw = value?;
    let seconds: i64 = raw.parse().ok()?;
    let timestamp = time::OffsetDateTime::from_unix_timestamp(seconds).ok()?;
    let format =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
            .ok()?;
    Some(timestamp.format(&format).ok()?)
}

pub fn current_datetime() -> String {
    let format = time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]")
        .unwrap_or_else(|_| time::format_description::parse("[year]-[month]-[day]").expect("format"));
    let now = time::OffsetDateTime::now_local().unwrap_or_else(|_| time::OffsetDateTime::now_utc());
    now.format(&format).unwrap_or_else(|_| "n/a".to_string())
}

pub fn format_duration(started_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let total = now.saturating_sub(started_at);
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;
    format!("{hours}h {minutes}m {seconds}s")
}
