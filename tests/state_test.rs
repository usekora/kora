use kora::state::{can_transition, checkpoint_for_stage, Checkpoint, Stage};

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

#[test]
fn test_run_state_set_error() {
    let mut state = RunState::new("test");
    state.set_error("something went wrong");
    assert_eq!(state.error, Some("something went wrong".to_string()));
    assert!(matches!(state.status, Stage::Failed(_)));
}

#[test]
fn test_run_state_increment_iteration() {
    let mut state = RunState::new("test");
    assert_eq!(state.current_iteration, 0);
    state.increment_iteration();
    assert_eq!(state.current_iteration, 1);
    state.increment_iteration();
    assert_eq!(state.current_iteration, 2);
}

#[test]
fn test_checkpoint_for_reviewing_with_after_researcher() {
    let checkpoints = vec![Checkpoint::AfterResearcher];
    let result = checkpoint_for_stage(&Stage::Reviewing, &checkpoints);
    assert_eq!(result, Some(Checkpoint::AfterResearcher));
}

#[test]
fn test_checkpoint_for_reviewing_without_after_researcher() {
    let checkpoints = vec![Checkpoint::AfterPlanner];
    let result = checkpoint_for_stage(&Stage::Reviewing, &checkpoints);
    assert_eq!(result, None);
}

#[test]
fn test_checkpoint_for_planning_with_after_review_loop() {
    let checkpoints = vec![Checkpoint::AfterReviewLoop];
    let result = checkpoint_for_stage(&Stage::Planning, &checkpoints);
    assert_eq!(result, Some(Checkpoint::AfterReviewLoop));
}

#[test]
fn test_checkpoint_for_implementing_with_after_planner() {
    let checkpoints = vec![Checkpoint::AfterPlanner];
    let result = checkpoint_for_stage(&Stage::Implementing, &checkpoints);
    assert_eq!(result, Some(Checkpoint::AfterPlanner));
}

#[test]
fn test_checkpoint_for_unrelated_stage_returns_none() {
    let checkpoints = vec![
        Checkpoint::AfterResearcher,
        Checkpoint::AfterReviewLoop,
        Checkpoint::AfterPlanner,
    ];
    let result = checkpoint_for_stage(&Stage::Judging, &checkpoints);
    assert_eq!(result, None);
}
