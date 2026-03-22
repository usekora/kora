use kora::pipeline::metrics::{estimate_tokens, RunMetrics};
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn test_new_metrics_empty() {
    let metrics = RunMetrics::new();
    assert!(metrics.invocations.is_empty());
    assert!(metrics.completed_at.is_none());
    // started_at should be set (not the epoch)
    assert!(metrics.started_at.timestamp() > 0);
}

#[test]
fn test_record_invocation() {
    let mut metrics = RunMetrics::new();
    let input = "What is Rust?";
    let output = "Rust is a systems programming language.";

    metrics.record(
        "researcher",
        "claude",
        Duration::from_secs(10),
        input,
        output,
    );

    assert_eq!(metrics.invocations.len(), 1);
    let inv = &metrics.invocations[0];
    assert_eq!(inv.agent_name, "researcher");
    assert_eq!(inv.provider, "claude");
    assert_eq!(inv.duration_secs, 10);
    assert_eq!(inv.estimated_input_tokens, estimate_tokens(input));
    assert_eq!(inv.estimated_output_tokens, estimate_tokens(output));
}

#[test]
fn test_estimate_tokens_empty() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn test_estimate_tokens_short() {
    // "hello" = 5 chars, 5.div_ceil(4) = 2
    assert_eq!(estimate_tokens("hello"), 2);
}

#[test]
fn test_estimate_tokens_exact() {
    // "12345678" = 8 chars, 8.div_ceil(4) = 2
    assert_eq!(estimate_tokens("12345678"), 2);
}

#[test]
fn test_total_duration() {
    let mut metrics = RunMetrics::new();
    metrics.record(
        "researcher",
        "claude",
        Duration::from_secs(45),
        "input1",
        "output1",
    );
    metrics.record(
        "planner",
        "claude",
        Duration::from_secs(30),
        "input2",
        "output2",
    );
    metrics.record(
        "implementor",
        "codex",
        Duration::from_secs(60),
        "input3",
        "output3",
    );

    assert_eq!(metrics.total_duration(), Duration::from_secs(135));
}

#[test]
fn test_total_estimated_tokens() {
    let mut metrics = RunMetrics::new();
    // "abcd" = 4 chars => 4.div_ceil(4) = 1 token each for input and output
    metrics.record("agent1", "claude", Duration::from_secs(10), "abcd", "abcd");
    // "abcdefgh" = 8 chars => 8.div_ceil(4) = 2 tokens each
    metrics.record(
        "agent2",
        "claude",
        Duration::from_secs(20),
        "abcdefgh",
        "abcdefgh",
    );

    // agent1: 1 + 1 = 2, agent2: 2 + 2 = 4, total = 6
    assert_eq!(metrics.total_estimated_tokens(), 6);
}

#[test]
fn test_save_and_load_roundtrip() {
    let dir = TempDir::new().unwrap();
    let mut metrics = RunMetrics::new();
    metrics.record(
        "researcher",
        "claude",
        Duration::from_secs(45),
        "Hello world, this is a test input.",
        "Here is the research output.",
    );
    metrics.record(
        "planner",
        "codex",
        Duration::from_secs(30),
        "Plan the work.",
        "Step 1: do this. Step 2: do that.",
    );
    metrics.complete();

    metrics.save(dir.path()).unwrap();
    let loaded = RunMetrics::load(dir.path()).unwrap();

    assert_eq!(loaded.invocations.len(), 2);
    assert_eq!(loaded.invocations[0].agent_name, "researcher");
    assert_eq!(loaded.invocations[1].agent_name, "planner");
    assert_eq!(loaded.invocations[0].duration_secs, 45);
    assert_eq!(loaded.invocations[1].duration_secs, 30);
    assert!(loaded.completed_at.is_some());
    assert_eq!(
        loaded.invocations[0].estimated_input_tokens,
        metrics.invocations[0].estimated_input_tokens
    );
    assert_eq!(
        loaded.invocations[1].estimated_output_tokens,
        metrics.invocations[1].estimated_output_tokens
    );
}

#[test]
fn test_summary_lines_not_empty() {
    let mut metrics = RunMetrics::new();
    metrics.record(
        "researcher",
        "claude",
        Duration::from_secs(45),
        "input",
        "output",
    );

    let lines = metrics.summary_lines();
    assert!(!lines.is_empty());
    assert!(lines[0].contains("invocations"));
    assert!(lines[1].contains("total time"));
    assert!(lines[2].contains("estimated tokens"));
}

#[test]
fn test_complete_sets_timestamp() {
    let mut metrics = RunMetrics::new();
    assert!(metrics.completed_at.is_none());

    metrics.complete();
    assert!(metrics.completed_at.is_some());
}
