use kora::state::{can_transition, Stage};

#[test]
fn test_researching_to_reviewing_is_valid() {
    assert!(can_transition(&Stage::Researching, &Stage::Reviewing));
}

#[test]
fn test_researching_to_security_auditing_is_valid() {
    assert!(can_transition(
        &Stage::Researching,
        &Stage::SecurityAuditing
    ));
}

#[test]
fn test_researching_to_implementing_is_invalid() {
    assert!(!can_transition(&Stage::Researching, &Stage::Implementing));
}

#[test]
fn test_judging_to_researching_is_valid_for_revise() {
    assert!(can_transition(&Stage::Judging, &Stage::Researching));
}

#[test]
fn test_judging_to_planning_is_valid_for_approve() {
    assert!(can_transition(&Stage::Judging, &Stage::Planning));
}

#[test]
fn test_awaiting_approval_wraps_next_stage() {
    let stage = Stage::AwaitingApproval(Box::new(Stage::Reviewing));
    assert!(can_transition(&stage, &Stage::Reviewing));
}

use kora::state::RunState;
use tempfile::TempDir;

#[test]
fn test_run_state_save_and_load() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let state = RunState::new("add dark mode support");
    state.save(runs_dir).unwrap();

    let loaded = RunState::load(runs_dir, &state.id).unwrap();
    assert_eq!(loaded.id, state.id);
    assert_eq!(loaded.request, "add dark mode support");
}
