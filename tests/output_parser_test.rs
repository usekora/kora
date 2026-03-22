use kora::agent::output_parser::{
    extract_json_object, extract_plan, parse_review, parse_security_review, parse_task_breakdown,
    parse_task_result, parse_test_strategy, parse_validation, parse_verdict, TaskStatus,
};

#[test]
fn test_parse_verdict_approve() {
    let text = r#"
Some reasoning here...

<!-- VERDICT -->
- REVIEWER_FINDING_1: DISMISSED
- REVIEWER_FINDING_2: DISMISSED
- OVERALL: APPROVE
- VALID_COUNT: 0
- DISMISSED_COUNT: 2
<!-- /VERDICT -->
"#;
    let verdict = parse_verdict(text).unwrap();
    assert_eq!(verdict.overall, "APPROVE");
    assert_eq!(verdict.valid_count, 0);
    assert_eq!(verdict.dismissed_count, 2);
}

#[test]
fn test_parse_verdict_revise() {
    let text = r#"
<!-- VERDICT -->
- REVIEWER_FINDING_1: VALID
- SECURITY_FINDING_1: DISMISSED
- OVERALL: REVISE
- VALID_COUNT: 1
- DISMISSED_COUNT: 1
<!-- /VERDICT -->
"#;
    let verdict = parse_verdict(text).unwrap();
    assert_eq!(verdict.overall, "REVISE");
    assert_eq!(verdict.valid_count, 1);
}

#[test]
fn test_parse_verdict_missing_markers_returns_none() {
    let text = "No structured output here";
    assert!(parse_verdict(text).is_none());
}

#[test]
fn test_parse_review_summary() {
    let text = r#"
<!-- REVIEW -->
- FINDING_1: HIGH No migration strategy
- FINDING_2: MEDIUM Missing error boundary
- FINDING_3: LOW Const enum suggestion
- TOTAL: 3 findings (1 high, 1 medium, 1 low)
<!-- /REVIEW -->
"#;
    let review = parse_review(text).unwrap();
    assert_eq!(review.findings.len(), 3);
    assert_eq!(review.findings[0].severity, "HIGH");
}

#[test]
fn test_parse_validation_pass() {
    let text = r#"
<!-- VALIDATION -->
- STATUS: PASS
- BLOCKING_ISSUES: 0
- MINOR_ISSUES: 1
- TEST_SUITE: 42 passed, 0 failed
- TYPE_CHECK: PASS
<!-- /VALIDATION -->
"#;
    let result = parse_validation(text).unwrap();
    assert!(result.passed);
    assert_eq!(result.blocking_issues, 0);
}

#[test]
fn test_parse_security_review_summary() {
    let text = r#"
Some analysis...

<!-- SECURITY -->
- FINDING_1: HIGH SQL injection in user input handler
- FINDING_2: MEDIUM Missing rate limiting on API
- TOTAL: 2 findings (1 high, 1 medium, 0 low)
<!-- /SECURITY -->
"#;
    let review = parse_security_review(text).unwrap();
    assert_eq!(review.findings.len(), 2);
    assert_eq!(review.findings[0].severity, "HIGH");
    assert_eq!(
        review.findings[0].title,
        "SQL injection in user input handler"
    );
    assert_eq!(review.findings[1].severity, "MEDIUM");
}

#[test]
fn test_parse_security_review_missing_markers() {
    let text = "No security markers here";
    assert!(parse_security_review(text).is_none());
}

#[test]
fn test_extract_plan() {
    let text = r#"
Here is some discussion...

<!-- PLAN -->
## Approach

Use a dark mode CSS variable system.

## Files to Change

- src/theme.ts: add dark mode variables
<!-- /PLAN -->

Some trailing text.
"#;
    let plan = extract_plan(text).unwrap();
    assert!(plan.contains("dark mode CSS variable system"));
    assert!(plan.contains("Files to Change"));
}

#[test]
fn test_extract_plan_missing_markers() {
    let text = "No plan markers here";
    assert!(extract_plan(text).is_none());
}

#[test]
fn test_parse_task_breakdown_valid() {
    let json = r#"{
        "tasks": [
            {
                "id": "T1",
                "title": "Add theme context",
                "description": "Create ThemeContext provider",
                "files": { "create": ["src/theme.ts"], "modify": [], "delete": [] },
                "depends_on": [],
                "estimated_complexity": "medium",
                "conflict_risk": []
            },
            {
                "id": "T2",
                "title": "Add CSS variables",
                "description": "Create CSS variable system",
                "files": { "create": ["src/vars.css"], "modify": [], "delete": [] },
                "depends_on": ["T1"],
                "estimated_complexity": "small"
            }
        ],
        "branch_strategy": "separate",
        "merge_order": ["T1", "T2"],
        "critical_path": ["T1", "T2"],
        "parallelism_summary": "1 then 1"
    }"#;
    let breakdown = parse_task_breakdown(json).unwrap();
    assert_eq!(breakdown.tasks.len(), 2);
    assert_eq!(breakdown.tasks[0].id, "T1");
    assert_eq!(breakdown.tasks[1].depends_on, vec!["T1"]);
    assert_eq!(breakdown.branch_strategy, "separate");
}

#[test]
fn test_parse_task_breakdown_malformed() {
    let json = "not valid json";
    assert!(parse_task_breakdown(json).is_err());
}

#[test]
fn test_parse_test_strategy_valid() {
    let json = r#"{
        "per_task": {
            "T1": {
                "unit_tests": [
                    {
                        "description": "Test theme toggle",
                        "file": "src/theme.test.ts",
                        "setup": "render ThemeProvider",
                        "expected": "theme changes on toggle",
                        "rationale": "catches broken toggle"
                    }
                ],
                "integration_tests": [],
                "edge_case_tests": []
            }
        },
        "post_merge": {
            "integration_tests": []
        },
        "testing_patterns": {
            "framework": "jest",
            "conventions": "describe/it blocks"
        }
    }"#;
    let strategy = parse_test_strategy(json).unwrap();
    assert!(strategy.per_task.contains_key("T1"));
    assert_eq!(strategy.per_task["T1"].unit_tests.len(), 1);
    assert_eq!(strategy.testing_patterns.framework, "jest");
}

#[test]
fn test_parse_test_strategy_malformed() {
    let json = "{}}}";
    assert!(parse_test_strategy(json).is_err());
}

#[test]
fn test_extract_json_object_from_mixed_text() {
    let text = r#"Here is the task breakdown:

{
  "tasks": [],
  "branch_strategy": "separate",
  "merge_order": [],
  "critical_path": [],
  "parallelism_summary": "none"
}

That's the breakdown."#;
    let json = extract_json_object(text).unwrap();
    assert!(json.starts_with('{'));
    assert!(json.ends_with('}'));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["branch_strategy"], "separate");
}

#[test]
fn test_extract_json_object_no_json() {
    let text = "no json here at all";
    assert!(extract_json_object(text).is_none());
}

#[test]
fn test_parse_task_result_complete() {
    let text = r#"## Status: COMPLETE

## Changes Made
- src/theme.ts: added dark mode toggle
- src/vars.css: created CSS variable system

## Tests
- 5 tests written, 5 passing, 0 failing

## Conflicts

## Out of Scope Observations
- index.ts could use cleanup
"#;
    let result = parse_task_result(text).unwrap();
    assert_eq!(result.status, TaskStatus::Complete);
    assert_eq!(result.changes.len(), 2);
    assert_eq!(result.tests_written, 5);
    assert_eq!(result.tests_passing, 5);
    assert_eq!(result.tests_failing, 0);
    assert_eq!(result.observations.len(), 1);
}

#[test]
fn test_parse_task_result_failed() {
    let text = r#"## Status: FAILED

## Changes Made
- src/theme.ts: partially implemented

## Tests
- 3 tests written, 1 passing, 2 failing

## Conflicts

## Out of Scope Observations
"#;
    let result = parse_task_result(text).unwrap();
    assert_eq!(result.status, TaskStatus::Failed);
    assert_eq!(result.tests_written, 3);
    assert_eq!(result.tests_passing, 1);
    assert_eq!(result.tests_failing, 2);
}

#[test]
fn test_parse_task_result_conflict() {
    let text = r#"## Status: CONFLICT

## Changes Made
- src/theme.ts: attempted changes

## Tests
- 0 tests written, 0 passing, 0 failing

## Conflicts
- src/shared.ts: conflicting changes from T1 and T3

## Out of Scope Observations
"#;
    let result = parse_task_result(text).unwrap();
    assert_eq!(result.status, TaskStatus::Conflict);
    assert_eq!(result.conflicts.len(), 1);
}

#[test]
fn test_parse_task_result_no_status() {
    let text = "some random text without status markers";
    assert!(parse_task_result(text).is_none());
}
