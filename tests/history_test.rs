use kora::state::{RunState, Stage};
use tempfile::TempDir;

#[test]
fn test_run_state_load_all_from_directory() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut state1 = RunState::new("add dark mode");
    state1.id = "hist1".to_string();
    state1.advance(Stage::Complete);
    state1.save(runs_dir).unwrap();

    let mut state2 = RunState::new("fix n+1 query");
    state2.id = "hist2".to_string();
    state2.set_error("review failed");
    state2.save(runs_dir).unwrap();

    let loaded1 = RunState::load(runs_dir, "hist1").unwrap();
    assert_eq!(loaded1.request, "add dark mode");
    assert_eq!(loaded1.status, Stage::Complete);

    let loaded2 = RunState::load(runs_dir, "hist2").unwrap();
    assert_eq!(loaded2.request, "fix n+1 query");
    assert!(matches!(loaded2.status, Stage::Failed(_)));
}

#[test]
fn test_run_state_timestamps_track_stages() {
    let mut state = RunState::new("test timestamps");
    assert!(state.timestamps.contains_key("created"));

    state.advance(Stage::Reviewing);
    assert!(state.timestamps.contains_key("reviewer"));

    state.advance(Stage::Judging);
    assert!(state.timestamps.contains_key("judge"));
}

#[test]
fn test_run_state_created_at_before_updated_at() {
    let mut state = RunState::new("ordering test");
    std::thread::sleep(std::time::Duration::from_millis(10));
    state.advance(Stage::Reviewing);
    assert!(state.updated_at >= state.created_at);
}

#[test]
fn test_run_state_load_nonexistent_fails() {
    let tmp = TempDir::new().unwrap();
    let result = RunState::load(tmp.path(), "nonexistent");
    assert!(result.is_err());
}

#[test]
fn test_completed_run_has_complete_status() {
    let mut state = RunState::new("completed run");
    state.advance(Stage::Complete);
    assert_eq!(state.status, Stage::Complete);
}

#[test]
fn test_failed_run_has_error_message() {
    let mut state = RunState::new("failed run");
    state.set_error("something went wrong");
    assert_eq!(state.error, Some("something went wrong".to_string()));
    match state.status {
        Stage::Failed(msg) => assert_eq!(msg, "something went wrong"),
        _ => panic!("expected Failed stage"),
    }
}
