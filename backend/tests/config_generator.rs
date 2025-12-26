use backend::config_gen::generate_server_config;

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
