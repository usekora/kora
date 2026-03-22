use kora::state::{can_transition, checkpoint_for_stage, Checkpoint, PipelineProfile, Stage};

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

// --- PipelineProfile tests ---

#[test]
fn test_pipeline_profile_trivial_skips_most_stages() {
    let p = PipelineProfile::Trivial;
    assert!(!p.has_review_loop());
    assert!(!p.has_planner());
    assert!(!p.has_test_architect());
    assert!(!p.has_code_review());
    assert!(!p.has_security_audit());
    assert!(!p.has_validation());
}

#[test]
fn test_pipeline_profile_simple_skips_review_and_test_architect() {
    let p = PipelineProfile::Simple;
    assert!(!p.has_review_loop());
    assert!(p.has_planner());
    assert!(!p.has_test_architect());
    assert!(p.has_code_review());
    assert!(!p.has_security_audit());
    assert!(p.has_validation());
}

#[test]
fn test_pipeline_profile_standard_has_all() {
    let p = PipelineProfile::Standard;
    assert!(p.has_review_loop());
    assert!(p.has_planner());
    assert!(p.has_test_architect());
    assert!(p.has_code_review());
    assert!(p.has_security_audit());
    assert!(p.has_validation());
}

#[test]
fn test_pipeline_profile_security_critical_has_all() {
    let p = PipelineProfile::SecurityCritical;
    assert!(p.has_review_loop());
    assert!(p.has_planner());
    assert!(p.has_test_architect());
    assert!(p.has_code_review());
    assert!(p.has_security_audit());
    assert!(p.has_validation());
}

#[test]
fn test_pipeline_profile_from_str() {
    assert_eq!(
        "trivial".parse::<PipelineProfile>(),
        Ok(PipelineProfile::Trivial)
    );
    assert_eq!(
        "simple".parse::<PipelineProfile>(),
        Ok(PipelineProfile::Simple)
    );
    assert_eq!(
        "standard".parse::<PipelineProfile>(),
        Ok(PipelineProfile::Standard)
    );
    assert_eq!(
        "security-critical".parse::<PipelineProfile>(),
        Ok(PipelineProfile::SecurityCritical)
    );
    assert_eq!(
        "security_critical".parse::<PipelineProfile>(),
        Ok(PipelineProfile::SecurityCritical)
    );
    assert!("banana".parse::<PipelineProfile>().is_err());
}

#[test]
fn test_pipeline_profile_display() {
    assert_eq!(PipelineProfile::Trivial.to_string(), "trivial");
    assert_eq!(PipelineProfile::Simple.to_string(), "simple");
    assert_eq!(PipelineProfile::Standard.to_string(), "standard");
    assert_eq!(
        PipelineProfile::SecurityCritical.to_string(),
        "security-critical"
    );
}

#[test]
fn test_pipeline_profile_default_is_standard() {
    assert_eq!(PipelineProfile::default(), PipelineProfile::Standard);
}

#[test]
fn test_run_state_with_pipeline_profile_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut state = RunState::new("add auth feature");
    state.pipeline_profile = Some(PipelineProfile::SecurityCritical);
    state.save(runs_dir).unwrap();

    let loaded = RunState::load(runs_dir, &state.id).unwrap();
    assert_eq!(
        loaded.pipeline_profile,
        Some(PipelineProfile::SecurityCritical)
    );
}

#[test]
fn test_run_state_without_pipeline_profile_loads_as_none() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    // Save a state without setting pipeline_profile (simulating old format)
    let state = RunState::new("old run");
    state.save(runs_dir).unwrap();

    let loaded = RunState::load(runs_dir, &state.id).unwrap();
    assert_eq!(loaded.pipeline_profile, None);
}

// --- New can_transition tests for profile paths ---

#[test]
fn test_researching_to_planning_is_valid_for_profiles_skipping_review() {
    assert!(can_transition(&Stage::Researching, &Stage::Planning));
}

#[test]
fn test_planning_to_implementing_is_valid_for_profiles_skipping_test_architect() {
    assert!(can_transition(&Stage::Planning, &Stage::Implementing));
}

#[test]
fn test_implementing_to_complete_is_valid_for_profiles_skipping_validation() {
    assert!(can_transition(&Stage::Implementing, &Stage::Complete));
}
