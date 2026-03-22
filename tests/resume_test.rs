use kora::state::{RunDirectory, RunState, Stage};
use tempfile::TempDir;

#[test]
fn test_list_interrupted_returns_non_complete_runs() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut state1 = RunState::new("add feature");
    state1.id = "run1".to_string();
    state1.save(runs_dir).unwrap();

    let mut state2 = RunState::new("fix bug");
    state2.id = "run2".to_string();
    state2.advance(Stage::Complete);
    state2.save(runs_dir).unwrap();

    let mut state3 = RunState::new("refactor auth");
    state3.id = "run3".to_string();
    state3.advance(Stage::Implementing);
    state3.save(runs_dir).unwrap();

    let interrupted = RunDirectory::list_interrupted(runs_dir).unwrap();
    let ids: Vec<&str> = interrupted.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&"run1"));
    assert!(ids.contains(&"run3"));
    assert!(!ids.contains(&"run2"));
}

#[test]
fn test_list_interrupted_empty_dir() {
    let tmp = TempDir::new().unwrap();
    let interrupted = RunDirectory::list_interrupted(tmp.path()).unwrap();
    assert!(interrupted.is_empty());
}

#[test]
fn test_list_interrupted_nonexistent_dir() {
    let tmp = TempDir::new().unwrap();
    let non_existent = tmp.path().join("does-not-exist");
    let interrupted = RunDirectory::list_interrupted(&non_existent).unwrap();
    assert!(interrupted.is_empty());
}

#[test]
fn test_list_interrupted_excludes_failed_runs() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut state = RunState::new("broken run");
    state.id = "fail1".to_string();
    state.set_error("something broke");
    state.save(runs_dir).unwrap();

    let interrupted = RunDirectory::list_interrupted(runs_dir).unwrap();
    assert!(interrupted.is_empty());
}

#[test]
fn test_list_interrupted_sorted_by_updated_desc() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut state1 = RunState::new("older run");
    state1.id = "old".to_string();
    state1.save(runs_dir).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let mut state2 = RunState::new("newer run");
    state2.id = "new".to_string();
    state2.advance(Stage::Implementing);
    state2.save(runs_dir).unwrap();

    let interrupted = RunDirectory::list_interrupted(runs_dir).unwrap();
    assert_eq!(interrupted.len(), 2);
    assert_eq!(interrupted[0].id, "new");
    assert_eq!(interrupted[1].id, "old");
}

#[test]
fn test_stage_label_for_resume_display() {
    assert_eq!(Stage::Researching.label(), "researcher");
    assert_eq!(Stage::Reviewing.label(), "reviewer");
    assert_eq!(Stage::Planning.label(), "planner");
    assert_eq!(Stage::Implementing.label(), "implementing");
    assert_eq!(Stage::Validating.label(), "validator");
    assert_eq!(Stage::Fixing.label(), "fixing");
    assert_eq!(Stage::Complete.label(), "complete");
}

#[test]
fn test_awaiting_approval_label() {
    let stage = Stage::AwaitingApproval(Box::new(Stage::Implementing));
    assert_eq!(stage.label(), "awaiting approval");
}
