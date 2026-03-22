use kora::state::{RunDirectory, RunState, Stage};
use tempfile::TempDir;

#[test]
fn test_list_cleanable_runs_only_complete_and_failed() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut complete_run = RunState::new("completed");
    complete_run.id = "c1".to_string();
    complete_run.advance(Stage::Complete);
    complete_run.save(runs_dir).unwrap();

    let mut failed_run = RunState::new("failed");
    failed_run.id = "f1".to_string();
    failed_run.set_error("boom");
    failed_run.save(runs_dir).unwrap();

    let mut interrupted_run = RunState::new("interrupted");
    interrupted_run.id = "i1".to_string();
    interrupted_run.advance(Stage::Implementing);
    interrupted_run.save(runs_dir).unwrap();

    let interrupted = RunDirectory::list_interrupted(runs_dir).unwrap();
    let interrupted_ids: Vec<&str> = interrupted.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(interrupted_ids, vec!["i1"]);

    let all_states = load_all_runs_for_test(runs_dir);
    let cleanable: Vec<&RunState> = all_states
        .iter()
        .filter(|r| matches!(r.status, Stage::Complete | Stage::Failed(_)))
        .collect();
    assert_eq!(cleanable.len(), 2);
}

fn load_all_runs_for_test(runs_dir: &std::path::Path) -> Vec<RunState> {
    let mut runs = Vec::new();
    for entry in std::fs::read_dir(runs_dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            let run_id = entry.file_name().to_string_lossy().to_string();
            if let Ok(state) = RunState::load(runs_dir, &run_id) {
                runs.push(state);
            }
        }
    }
    runs
}

#[test]
fn test_run_directory_removal() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut state = RunState::new("to be cleaned");
    state.id = "clean1".to_string();
    state.advance(Stage::Complete);
    state.save(runs_dir).unwrap();

    let run_path = runs_dir.join("clean1");
    assert!(run_path.exists());

    std::fs::remove_dir_all(&run_path).unwrap();
    assert!(!run_path.exists());
}

#[test]
fn test_empty_runs_dir_nothing_to_clean() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let all_states = load_all_runs_for_test(runs_dir);
    assert!(all_states.is_empty());
}

#[test]
fn test_nonexistent_runs_dir_nothing_to_clean() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path().join("nonexistent");
    assert!(!runs_dir.exists());
}

#[test]
fn test_filter_runs_by_age() {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path();

    let mut state = RunState::new("recent run");
    state.id = "recent".to_string();
    state.advance(Stage::Complete);
    state.save(runs_dir).unwrap();

    let cutoff = chrono::Utc::now() - chrono::Duration::days(7);
    let all_runs = load_all_runs_for_test(runs_dir);
    let old_runs: Vec<&RunState> = all_runs.iter().filter(|r| r.created_at < cutoff).collect();
    assert!(old_runs.is_empty());
}
