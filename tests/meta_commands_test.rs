use kora::cli::meta_commands::{parse_meta_command, MetaCommand};

#[test]
fn test_parse_status_command() {
    assert_eq!(parse_meta_command("/status"), MetaCommand::Status);
}

#[test]
fn test_parse_config_command() {
    assert_eq!(parse_meta_command("/config"), MetaCommand::Configure);
}

#[test]
fn test_parse_configure_command() {
    assert_eq!(parse_meta_command("/configure"), MetaCommand::Configure);
}

#[test]
fn test_parse_verbose_is_regular_input() {
    assert_eq!(
        parse_meta_command("/verbose"),
        MetaCommand::None("/verbose".to_string())
    );
}

#[test]
fn test_parse_help_command() {
    assert_eq!(parse_meta_command("/help"), MetaCommand::Help);
}

#[test]
fn test_parse_quit_command() {
    assert_eq!(parse_meta_command("/quit"), MetaCommand::Quit);
}

#[test]
fn test_parse_exit_command() {
    assert_eq!(parse_meta_command("/exit"), MetaCommand::Quit);
}

#[test]
fn test_parse_regular_input() {
    let result = parse_meta_command("add dark mode support");
    assert_eq!(
        result,
        MetaCommand::None("add dark mode support".to_string())
    );
}

#[test]
fn test_parse_unknown_slash_command() {
    let result = parse_meta_command("/unknown");
    assert_eq!(result, MetaCommand::None("/unknown".to_string()));
}

#[test]
fn test_parse_empty_input() {
    let result = parse_meta_command("");
    assert_eq!(result, MetaCommand::None(String::new()));
}

#[test]
fn test_parse_whitespace_trimmed() {
    assert_eq!(parse_meta_command("  /status  "), MetaCommand::Status);
}

#[test]
fn test_parse_regular_input_trimmed() {
    let result = parse_meta_command("  fix the bug  ");
    assert_eq!(result, MetaCommand::None("fix the bug".to_string()));
}

#[test]
fn test_slash_in_middle_is_not_command() {
    let result = parse_meta_command("fix /the bug");
    assert_eq!(result, MetaCommand::None("fix /the bug".to_string()));
}

#[test]
fn test_parse_bare_exit() {
    assert_eq!(parse_meta_command("exit"), MetaCommand::Quit);
}

#[test]
fn test_parse_bare_quit() {
    assert_eq!(parse_meta_command("quit"), MetaCommand::Quit);
}

#[test]
fn test_parse_clear_command() {
    assert_eq!(parse_meta_command("/clear"), MetaCommand::Clear);
}

#[test]
fn test_parse_bare_clear() {
    assert_eq!(parse_meta_command("clear"), MetaCommand::Clear);
}
