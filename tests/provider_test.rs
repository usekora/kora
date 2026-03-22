use kora::provider::ProviderKind;

#[test]
fn test_provider_kind_cli_name() {
    assert_eq!(ProviderKind::Claude.cli_name(), "claude");
    assert_eq!(ProviderKind::Codex.cli_name(), "codex");
}

#[test]
fn test_provider_kind_autonomous_flags() {
    let flags = ProviderKind::Claude.autonomous_flags();
    assert!(flags.contains(&"--dangerously-skip-permissions"));

    let flags = ProviderKind::Codex.autonomous_flags();
    assert!(flags.contains(&"--approval-mode"));
    assert!(flags.contains(&"full-auto"));
}

#[test]
fn test_provider_kind_has_non_interactive_flags() {
    let flags = ProviderKind::Claude.non_interactive_flags();
    assert!(flags.contains(&"--print"));

    let flags = ProviderKind::Codex.non_interactive_flags();
    assert!(flags.contains(&"--quiet"));
}
