use std::collections::HashMap;

use kora::agent::output_parser::{
    Task, TaskBreakdown, TaskFiles, TaskResult, TaskStatus, TestStrategy,
    TestingPatterns, PostMergeTests,
};
use kora::config::Config;
use kora::pipeline::implementation::{
    ImplementationFleet, ImplementationTaskStatus, TaskEvent,
};
use kora::state::RunDirectory;
use tempfile::TempDir;

fn make_task(id: &str, depends_on: Vec<&str>) -> Task {
    Task {
        id: id.to_string(),
        title: format!("Task {}", id),
        description: format!("Do work for {}", id),
        files: TaskFiles {
            create: vec![],
            modify: vec![],
            delete: vec![],
        },
        depends_on: depends_on.into_iter().map(String::from).collect(),
        estimated_complexity: "small".to_string(),
        conflict_risk: vec![],
    }
}

fn make_breakdown(tasks: Vec<Task>) -> TaskBreakdown {
    let ids: Vec<String> = tasks.iter().map(|t| t.id.clone()).collect();
    TaskBreakdown {
        tasks,
        branch_strategy: "separate".to_string(),
        merge_order: ids,
        critical_path: vec![],
        parallelism_summary: String::new(),
    }
}

fn make_test_strategy() -> TestStrategy {
    TestStrategy {
        per_task: HashMap::new(),
        post_merge: PostMergeTests {
            integration_tests: vec![],
        },
        testing_patterns: TestingPatterns {
            framework: "cargo test".to_string(),
            conventions: "#[test]".to_string(),
        },
    }
}

fn make_fleet(tasks: Vec<Task>) -> (ImplementationFleet, TempDir) {
    let tmp = TempDir::new().unwrap();
    let runs_dir = tmp.path().join("runs");
    std::fs::create_dir_all(&runs_dir).unwrap();
    let run_dir = RunDirectory::new(&runs_dir, "test-run");
    run_dir.create_structure().unwrap();
    let config = Config::default();
    let breakdown = make_breakdown(tasks);
    let strategy = make_test_strategy();
    let fleet = ImplementationFleet::new(config, breakdown, strategy, tmp.path(), &run_dir);
    (fleet, tmp)
}

#[test]
fn test_ready_tasks_returns_independent_tasks() {
    let (fleet, _tmp) = make_fleet(vec![
        make_task("T1", vec![]),
        make_task("T2", vec![]),
        make_task("T3", vec!["T1"]),
    ]);
    let ready = fleet.ready_tasks();
    assert!(ready.contains(&"T1".to_string()));
    assert!(ready.contains(&"T2".to_string()));
    assert!(!ready.contains(&"T3".to_string()));
}

#[test]
fn test_blocked_task_not_ready() {
    let (fleet, _tmp) = make_fleet(vec![
        make_task("T1", vec![]),
        make_task("T2", vec!["T1"]),
    ]);
    let ready = fleet.ready_tasks();
    assert_eq!(ready, vec!["T1"]);
}

#[test]
fn test_check_unblocked_transitions_blocked_to_pending() {
    let (mut fleet, _tmp) = make_fleet(vec![
        make_task("T1", vec![]),
        make_task("T2", vec!["T1"]),
    ]);

    let event = TaskEvent::Completed {
        task_id: "T1".to_string(),
        result: TaskResult {
            status: TaskStatus::Complete,
            changes: vec![],
            tests_written: 0,
            tests_passing: 0,
            tests_failing: 0,
            conflicts: vec![],
            observations: vec![],
        },
        duration_secs: 10,
        files_changed: 1,
    };
    fleet.handle_event(&event);

    let unblocked = fleet.check_unblocked();
    assert_eq!(unblocked, vec!["T2"]);
    assert!(fleet.ready_tasks().contains(&"T2".to_string()));
}

#[test]
fn test_is_done_false_when_pending() {
    let (fleet, _tmp) = make_fleet(vec![make_task("T1", vec![])]);
    assert!(!fleet.is_done());
}

#[test]
fn test_is_done_true_when_all_complete() {
    let (mut fleet, _tmp) = make_fleet(vec![make_task("T1", vec![])]);
    fleet.handle_event(&TaskEvent::Completed {
        task_id: "T1".to_string(),
        result: TaskResult {
            status: TaskStatus::Complete,
            changes: vec![],
            tests_written: 0,
            tests_passing: 0,
            tests_failing: 0,
            conflicts: vec![],
            observations: vec![],
        },
        duration_secs: 5,
        files_changed: 2,
    });
    assert!(fleet.is_done());
}

#[test]
fn test_is_done_true_when_all_failed() {
    let (mut fleet, _tmp) = make_fleet(vec![make_task("T1", vec![])]);
    fleet.handle_event(&TaskEvent::Failed {
        task_id: "T1".to_string(),
        error: "boom".to_string(),
        attempts: 1,
    });
    assert!(fleet.is_done());
}

#[test]
fn test_handle_event_failed_sets_status() {
    let (mut fleet, _tmp) = make_fleet(vec![make_task("T1", vec![])]);
    fleet.handle_event(&TaskEvent::Failed {
        task_id: "T1".to_string(),
        error: "provider crashed".to_string(),
        attempts: 2,
    });
    let state = &fleet.task_states()["T1"];
    assert!(matches!(
        state.status,
        ImplementationTaskStatus::Failed { .. }
    ));
}

#[test]
fn test_handle_event_conflict_sets_status() {
    let (mut fleet, _tmp) = make_fleet(vec![make_task("T1", vec![])]);
    fleet.handle_event(&TaskEvent::Completed {
        task_id: "T1".to_string(),
        result: TaskResult {
            status: TaskStatus::Conflict,
            changes: vec![],
            tests_written: 0,
            tests_passing: 0,
            tests_failing: 0,
            conflicts: vec!["src/shared.ts".to_string()],
            observations: vec![],
        },
        duration_secs: 8,
        files_changed: 0,
    });
    let state = &fleet.task_states()["T1"];
    assert!(matches!(
        state.status,
        ImplementationTaskStatus::Conflict { .. }
    ));
}

#[test]
fn test_running_count_starts_at_zero() {
    let (fleet, _tmp) = make_fleet(vec![make_task("T1", vec![])]);
    assert_eq!(fleet.running_count(), 0);
}

#[test]
fn test_dependency_chain_only_root_is_ready() {
    let (fleet, _tmp) = make_fleet(vec![
        make_task("T1", vec![]),
        make_task("T2", vec!["T1"]),
        make_task("T3", vec!["T2"]),
    ]);
    let ready = fleet.ready_tasks();
    assert_eq!(ready, vec!["T1"]);
}

#[test]
fn test_total_tasks() {
    let (fleet, _tmp) = make_fleet(vec![
        make_task("T1", vec![]),
        make_task("T2", vec![]),
        make_task("T3", vec!["T1"]),
    ]);
    assert_eq!(fleet.total_tasks(), 3);
}

#[test]
fn test_failed_tasks_returns_failed_ids() {
    let (mut fleet, _tmp) = make_fleet(vec![
        make_task("T1", vec![]),
        make_task("T2", vec![]),
    ]);
    fleet.handle_event(&TaskEvent::Failed {
        task_id: "T2".to_string(),
        error: "timeout".to_string(),
        attempts: 1,
    });
    fleet.handle_event(&TaskEvent::Completed {
        task_id: "T1".to_string(),
        result: TaskResult {
            status: TaskStatus::Complete,
            changes: vec![],
            tests_written: 0,
            tests_passing: 0,
            tests_failing: 0,
            conflicts: vec![],
            observations: vec![],
        },
        duration_secs: 5,
        files_changed: 1,
    });
    let failed = fleet.failed_tasks();
    assert_eq!(failed, vec!["T2"]);
}

#[test]
fn test_merge_order_matches_breakdown() {
    let (fleet, _tmp) = make_fleet(vec![
        make_task("T1", vec![]),
        make_task("T2", vec!["T1"]),
    ]);
    assert_eq!(fleet.merge_order(), &["T1", "T2"]);
}
