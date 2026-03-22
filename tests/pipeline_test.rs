use kora::config::Config;
use kora::pipeline::orchestrator::effective_checkpoints;
use kora::pipeline::orchestrator::PipelineOptions;
use kora::state::Checkpoint;

fn make_options(yolo: bool, careful: bool) -> PipelineOptions {
    PipelineOptions {
        request: "test".to_string(),
        yolo,
        careful,
        dry_run: false,
        provider_override: None,
        resume_run_id: None,
    }
}

#[test]
fn test_effective_checkpoints_yolo_is_empty() {
    let config = Config::default();
    let options = make_options(true, false);
    let checkpoints = effective_checkpoints(&config, &options);
    assert!(checkpoints.is_empty());
}

#[test]
fn test_effective_checkpoints_careful_has_all() {
    let config = Config::default();
    let options = make_options(false, true);
    let checkpoints = effective_checkpoints(&config, &options);
    assert_eq!(checkpoints.len(), 4);
    assert!(checkpoints.contains(&Checkpoint::AfterResearcher));
    assert!(checkpoints.contains(&Checkpoint::AfterReviewLoop));
    assert!(checkpoints.contains(&Checkpoint::AfterPlanner));
    assert!(checkpoints.contains(&Checkpoint::AfterImplementation));
}

#[test]
fn test_effective_checkpoints_default_uses_config() {
    let config = Config::default();
    let options = make_options(false, false);
    let checkpoints = effective_checkpoints(&config, &options);
    assert_eq!(checkpoints, config.checkpoints);
}
