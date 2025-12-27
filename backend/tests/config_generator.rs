use backend::config_gen::generate_server_config;
use backend::defaults::{apply_default_server_json_settings, apply_profile_overrides};
use backend::storage::AppSettings;
use backend::models::ServerProfile;

#[test]
fn overwrites_scenario_and_mods() {
    let mods = vec!["AAA".to_string(), "BBB".to_string(), "AAA".to_string()];
    let config = generate_server_config("{TEST}Missions/Example.conf", &mods, Some("Test"))
        .expect("config generation failed");

    let game = config.get("game").and_then(|value| value.as_object()).expect("missing game");
    assert_eq!(game.get("scenarioId").and_then(|value| value.as_str()), Some("{TEST}Missions/Example.conf"));
    assert_eq!(game.get("name").and_then(|value| value.as_str()), Some("Test"));

    let mods_value = game.get("mods").and_then(|value| value.as_array()).expect("missing mods");
    let mod_ids: Vec<&str> = mods_value
        .iter()
        .filter_map(|entry| entry.get("modId"))
        .filter_map(|value| value.as_str())
        .collect();

    assert_eq!(mod_ids, vec!["AAA", "BBB"]);
}

#[test]
fn applies_settings_defaults_and_profile_overrides() {
    let mut settings = AppSettings::default();
    settings.server_json_defaults = serde_json::json!({
        "bindPort": 4000,
        "game": { "maxPlayers": 10 }
    });
    settings.server_json_enabled.insert("bindPort".to_string(), true);
    settings.server_json_enabled.insert("game.maxPlayers".to_string(), true);

    let profile = ServerProfile {
        profile_id: "test".to_string(),
        display_name: "Test".to_string(),
        workshop_url: "url".to_string(),
        root_mod_id: None,
        selected_scenario_id_path: Some("{TEST}Missions/Example.conf".to_string()),
        dependency_mod_ids: Vec::new(),
        optional_mod_ids: Vec::new(),
        load_session_save: false,
        server_path_override: None,
        workshop_path_override: None,
        mod_path_override: None,
        server_json_overrides: serde_json::json!({
            "game": { "maxPlayers": 24 }
        }),
        server_json_override_enabled: std::collections::HashMap::from([
            ("game.maxPlayers".to_string(), true),
        ]),
        generated_config_path: None,
        last_resolved_at: None,
        last_resolve_hash: None,
    };

    let mut config = generate_server_config(
        "{TEST}Missions/Example.conf",
        &[],
        Some("Test"),
    )
    .expect("base config failed");

    apply_default_server_json_settings(&mut config, &settings);
    apply_profile_overrides(&mut config, &profile).expect("apply overrides");

    let bind_port = config.get("bindPort").and_then(|value| value.as_f64()).unwrap_or(0.0);
    assert_eq!(bind_port, 4000.0);

    let max_players = config
        .get("game")
        .and_then(|value| value.get("maxPlayers"))
        .and_then(|value| value.as_f64())
        .unwrap_or(0.0);
    assert_eq!(max_players, 24.0);
}
