use kora::agent::output_parser::{Task, TaskBreakdown, TaskFiles};
use kora::pipeline::planner::validate_breakdown;

fn make_task(id: &str, depends_on: Vec<&str>) -> Task {
    Task {
        id: id.to_string(),
        title: format!("Task {}", id),
        description: format!("Description for {}", id),
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

fn make_breakdown(tasks: Vec<Task>, merge_order: Vec<&str>) -> TaskBreakdown {
    TaskBreakdown {
        tasks,
        branch_strategy: "separate".to_string(),
        merge_order: merge_order.into_iter().map(String::from).collect(),
        critical_path: vec![],
        parallelism_summary: String::new(),
    }
}

#[test]
fn test_validate_breakdown_valid() {
    let breakdown = make_breakdown(
        vec![make_task("T1", vec![]), make_task("T2", vec!["T1"])],
        vec!["T1", "T2"],
    );
    assert!(validate_breakdown(&breakdown).is_ok());
}

#[test]
fn test_validate_breakdown_duplicate_ids() {
    let breakdown = make_breakdown(
        vec![make_task("T1", vec![]), make_task("T1", vec![])],
        vec!["T1"],
    );
    let err = validate_breakdown(&breakdown).unwrap_err();
    assert!(err.to_string().contains("duplicate"));
}

#[test]
fn test_validate_breakdown_missing_dependency() {
    let breakdown = make_breakdown(vec![make_task("T1", vec!["T99"])], vec!["T1"]);
    let err = validate_breakdown(&breakdown).unwrap_err();
    assert!(err.to_string().contains("non-existent task T99"));
}

#[test]
fn test_validate_breakdown_self_dependency() {
    let breakdown = make_breakdown(vec![make_task("T1", vec!["T1"])], vec!["T1"]);
    let err = validate_breakdown(&breakdown).unwrap_err();
    assert!(err.to_string().contains("depends on itself"));
}

#[test]
fn test_validate_breakdown_bad_merge_order() {
    let breakdown = make_breakdown(vec![make_task("T1", vec![])], vec!["T1", "T99"]);
    let err = validate_breakdown(&breakdown).unwrap_err();
    assert!(err
        .to_string()
        .contains("merge_order references non-existent task T99"));
}
