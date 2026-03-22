use kora::config::Config;

#[test]
fn test_default_config_has_claude_provider() {
    let config = Config::default();
    assert_eq!(config.default_provider, "claude");
}

#[test]
fn test_default_config_has_checkpoints() {
    let config = Config::default();
    assert!(config
        .checkpoints
        .contains(&kora::state::Checkpoint::AfterResearcher));
    assert!(config
        .checkpoints
        .contains(&kora::state::Checkpoint::AfterPlanner));
}

#[test]
fn test_config_roundtrip_yaml() {
    let config = Config::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(config, parsed);
}
