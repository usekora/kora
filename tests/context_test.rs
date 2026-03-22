use kora::agent::output_parser::{Task, TaskFiles};
use kora::pipeline::context;
use tempfile::TempDir;

#[test]
fn test_build_researcher_prompt_includes_request() {
    let result = context::build_researcher_prompt("add dark mode support", None).unwrap();
    assert!(result.prompt.contains("add dark mode support"));
}

#[test]
fn test_build_researcher_prompt_includes_base_prompt() {
    let result = context::build_researcher_prompt("test request", None).unwrap();
    assert!(result.prompt.contains("senior software architect"));
}

#[test]
fn test_build_researcher_prompt_includes_custom_instructions() {
    let result = context::build_researcher_prompt("test", Some("Always use TypeScript")).unwrap();
    assert!(result.prompt.contains("Always use TypeScript"));
    assert!(result.prompt.contains("Additional Instructions"));
}

#[test]
fn test_build_reviewer_prompt_includes_plan() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("test-run");
    std::fs::create_dir_all(run_dir.join("context")).unwrap();
    std::fs::write(
        run_dir.join("context").join("researcher-plan.md"),
        "## My Plan\nDo the thing",
    )
    .unwrap();

    let result =
        context::build_reviewer_prompt(&run_dir, 1, "add feature X", tmp.path(), None).unwrap();

    assert!(result.prompt.contains("Do the thing"));
    assert!(result.prompt.contains("add feature X"));
}

#[test]
fn test_build_reviewer_prompt_includes_previous_iterations() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("test-run");
    std::fs::create_dir_all(run_dir.join("context")).unwrap();
    std::fs::create_dir_all(run_dir.join("reviews").join("iteration-1")).unwrap();
    std::fs::write(
        run_dir.join("context").join("researcher-plan.md"),
        "plan content",
    )
    .unwrap();
    std::fs::write(
        run_dir
            .join("reviews")
            .join("iteration-1")
            .join("review.md"),
        "previous review findings",
    )
    .unwrap();
    std::fs::write(
        run_dir
            .join("reviews")
            .join("iteration-1")
            .join("judgment.md"),
        "previous judgment",
    )
    .unwrap();

    let result =
        context::build_reviewer_prompt(&run_dir, 2, "add feature X", tmp.path(), None).unwrap();

    assert!(result.prompt.contains("previous review findings"));
    assert!(result.prompt.contains("previous judgment"));
}

#[test]
fn test_build_judge_prompt_includes_review_and_security() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("test-run");
    std::fs::create_dir_all(run_dir.join("context")).unwrap();
    std::fs::create_dir_all(run_dir.join("reviews").join("iteration-1")).unwrap();
    std::fs::write(
        run_dir.join("context").join("researcher-plan.md"),
        "the plan",
    )
    .unwrap();
    std::fs::write(
        run_dir
            .join("reviews")
            .join("iteration-1")
            .join("review.md"),
        "reviewer findings here",
    )
    .unwrap();
    std::fs::write(
        run_dir
            .join("reviews")
            .join("iteration-1")
            .join("security-audit.md"),
        "security findings here",
    )
    .unwrap();

    let result = context::build_judge_prompt(&run_dir, 1, "fix the bug", tmp.path(), None).unwrap();

    assert!(result.prompt.contains("reviewer findings here"));
    assert!(result.prompt.contains("security findings here"));
    assert!(result.prompt.contains("the plan"));
}

#[test]
fn test_build_security_prompt_includes_plan() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("test-run");
    std::fs::create_dir_all(run_dir.join("context")).unwrap();
    std::fs::write(
        run_dir.join("context").join("researcher-plan.md"),
        "plan with auth changes",
    )
    .unwrap();

    let result =
        context::build_security_prompt(&run_dir, 1, "add auth endpoint", tmp.path(), None).unwrap();

    assert!(result.prompt.contains("plan with auth changes"));
    assert!(result.prompt.contains("plan security auditor"));
}

#[test]
fn test_build_researcher_revision_prompt_includes_findings() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("test-run");
    std::fs::create_dir_all(run_dir.join("context")).unwrap();
    std::fs::create_dir_all(run_dir.join("reviews").join("iteration-1")).unwrap();
    std::fs::write(
        run_dir.join("context").join("researcher-plan.md"),
        "original plan",
    )
    .unwrap();
    std::fs::write(
        run_dir
            .join("reviews")
            .join("iteration-1")
            .join("judgment.md"),
        "FINDING_1: VALID - missing migration",
    )
    .unwrap();

    let result = context::build_researcher_revision_prompt(&run_dir, 1, tmp.path(), None).unwrap();

    assert!(result.prompt.contains("original plan"));
    assert!(result.prompt.contains("missing migration"));
    assert!(result.prompt.contains("Revision Mode"));
}

#[test]
fn test_build_planner_prompt_includes_plan_and_summary() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("test-run");
    std::fs::create_dir_all(run_dir.join("context")).unwrap();
    std::fs::write(
        run_dir.join("context").join("researcher-plan.md"),
        "the approved plan",
    )
    .unwrap();
    std::fs::write(
        run_dir.join("context").join("codebase-summary.md"),
        "monorepo with 3 services",
    )
    .unwrap();

    let result =
        context::build_planner_prompt(&run_dir, "add dark mode", tmp.path(), None).unwrap();

    assert!(result.prompt.contains("the approved plan"));
    assert!(result.prompt.contains("monorepo with 3 services"));
    assert!(result.prompt.contains("add dark mode"));
    assert!(result.prompt.contains("engineering manager"));
}

#[test]
fn test_build_test_architect_prompt_includes_task_breakdown() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path().join("test-run");
    std::fs::create_dir_all(run_dir.join("context")).unwrap();
    std::fs::create_dir_all(run_dir.join("plan")).unwrap();
    std::fs::write(
        run_dir.join("context").join("researcher-plan.md"),
        "plan content",
    )
    .unwrap();
    std::fs::write(
        run_dir.join("plan").join("task-breakdown.json"),
        r#"{"tasks": []}"#,
    )
    .unwrap();

    let result =
        context::build_test_architect_prompt(&run_dir, "fix the bug", tmp.path(), None).unwrap();

    assert!(result.prompt.contains("plan content"));
    assert!(result.prompt.contains(r#""tasks": []"#));
    assert!(result.prompt.contains("QA architect"));
}

#[test]
fn test_build_implementor_prompt_includes_task_details() {
    let task = Task {
        id: "T1".to_string(),
        title: "Add theme context".to_string(),
        description: "Create a ThemeContext provider component".to_string(),
        files: TaskFiles {
            create: vec!["src/theme.ts".to_string()],
            modify: vec!["src/app.ts".to_string()],
            delete: vec![],
        },
        depends_on: vec![],
        estimated_complexity: "medium".to_string(),
        conflict_risk: vec![],
    };

    let result = context::build_implementor_prompt(&task, "test spec here").unwrap();

    assert!(result.contains("T1"));
    assert!(result.contains("Add theme context"));
    assert!(result.contains("ThemeContext provider"));
    assert!(result.contains("src/theme.ts"));
    assert!(result.contains("src/app.ts"));
    assert!(result.contains("test spec here"));
    assert!(result.contains("senior software developer"));
}

#[test]
fn test_build_implementor_prompt_empty_test_spec() {
    let task = Task {
        id: "T2".to_string(),
        title: "Simple task".to_string(),
        description: "Do something simple".to_string(),
        files: TaskFiles {
            create: vec![],
            modify: vec![],
            delete: vec![],
        },
        depends_on: vec![],
        estimated_complexity: "small".to_string(),
        conflict_risk: vec![],
    };

    let result = context::build_implementor_prompt(&task, "").unwrap();
    assert!(result.contains("No specific test requirements"));
}
