use serde_json::Value;
use std::collections::HashSet;

const BASELINE_CONFIG: &str = include_str!("../assets/server.sample.json");

pub fn baseline_config() -> &'static str {
    BASELINE_CONFIG
}

pub fn generate_server_config(
    scenario_id: &str,
    mod_ids: &[String],
    display_name: Option<&str>,
) -> Result<Value, String> {
    let mut root: Value = serde_json::from_str(BASELINE_CONFIG)
        .map_err(|err| format!("failed to parse baseline config: {err}"))?;

    apply_game_overrides(&mut root, scenario_id, mod_ids, display_name)?;

    Ok(root)
}

pub fn apply_game_overrides(
    config: &mut Value,
    scenario_id: &str,
    mod_ids: &[String],
    display_name: Option<&str>,
) -> Result<(), String> {
    let game = config
        .get_mut("game")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| "baseline config missing game object".to_string())?;

    game.insert(
        "scenarioId".to_string(),
        Value::String(scenario_id.to_string()),
    );

    if let Some(name) = display_name {
        game.insert("name".to_string(), Value::String(name.to_string()));
    }

    let mods = dedupe_mod_ids(mod_ids)
        .into_iter()
        .map(|id| {
            let mut mod_entry = serde_json::Map::new();
            mod_entry.insert("modId".to_string(), Value::String(id));
            Value::Object(mod_entry)
        })
        .collect::<Vec<_>>();

    game.insert("mods".to_string(), Value::Array(mods));
    Ok(())
}

fn dedupe_mod_ids(mod_ids: &[String]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for id in mod_ids {
        if seen.insert(id.clone()) {
            result.push(id.clone());
        }
    }
    result
}
