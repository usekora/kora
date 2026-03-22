use kora::agent::output_parser::parse_validation;
use kora::pipeline::validation::extract_required_fixes;

#[test]
fn test_parse_validation_pass() {
    let text = r#"
Everything looks good.

<!-- VALIDATION -->
- STATUS: PASS
- BLOCKING_ISSUES: 0
- MINOR_ISSUES: 2
- TEST_SUITE: 45 passed, 0 failed
- TYPE_CHECK: PASS
<!-- /VALIDATION -->
"#;
    let result = parse_validation(text).unwrap();
    assert!(result.passed);
    assert_eq!(result.blocking_issues, 0);
    assert_eq!(result.minor_issues, 2);
    assert_eq!(result.tests_passed, 45);
    assert_eq!(result.tests_failed, 0);
    assert!(result.type_check_passed);
}

#[test]
fn test_parse_validation_fail() {
    let text = r#"
Found issues.

<!-- VALIDATION -->
- STATUS: FAIL
- BLOCKING_ISSUES: 2
- MINOR_ISSUES: 1
- TEST_SUITE: 40 passed, 3 failed
- TYPE_CHECK: FAIL
<!-- /VALIDATION -->

## Required Fixes

### Fix 1: Missing export
**Severity:** BLOCKING
**File:** src/auth.ts
**Expected:** AuthProvider exported
**Actual:** Not exported
**Fix:** Add export to index.ts
"#;
    let result = parse_validation(text).unwrap();
    assert!(!result.passed);
    assert_eq!(result.blocking_issues, 2);
    assert_eq!(result.tests_failed, 3);
    assert!(!result.type_check_passed);
}

#[test]
fn test_parse_validation_missing_markers() {
    let text = "no structured output here";
    assert!(parse_validation(text).is_none());
}

#[test]
fn test_parse_validation_case_insensitive_status() {
    let text = r#"
<!-- VALIDATION -->
- STATUS: pass
- BLOCKING_ISSUES: 0
- MINOR_ISSUES: 0
- TEST_SUITE: 10 passed, 0 failed
- TYPE_CHECK: pass
<!-- /VALIDATION -->
"#;
    let result = parse_validation(text).unwrap();
    assert!(result.passed);
    assert!(result.type_check_passed);
}

#[test]
fn test_extract_required_fixes_single_fix() {
    let text = r#"
Some preamble.

## Required Fixes

### Fix 1: Missing export
**Severity:** BLOCKING
**File:** src/auth.ts
**Expected:** AuthProvider exported
**Actual:** Not exported
**Fix:** Add export to index.ts
"#;
    let fixes = extract_required_fixes(text);
    assert_eq!(fixes.len(), 1);
    assert!(fixes[0].contains("Missing export"));
    assert!(fixes[0].contains("src/auth.ts"));
}

#[test]
fn test_extract_required_fixes_multiple_fixes() {
    let text = r#"
## Required Fixes

### Fix 1: Missing export
**Severity:** BLOCKING
**File:** src/auth.ts
**Fix:** Add export

### Fix 2: Wrong return type
**Severity:** BLOCKING
**File:** src/api.ts
**Fix:** Change return type
"#;
    let fixes = extract_required_fixes(text);
    assert_eq!(fixes.len(), 2);
    assert!(fixes[0].contains("Missing export"));
    assert!(fixes[1].contains("Wrong return type"));
}

#[test]
fn test_extract_required_fixes_no_fixes() {
    let text = "No fixes section at all.";
    let fixes = extract_required_fixes(text);
    assert!(fixes.is_empty());
}

#[test]
fn test_extract_required_fixes_empty_section() {
    let text = "## Required Fixes\n\n## Other Section\n";
    let fixes = extract_required_fixes(text);
    assert!(fixes.is_empty());
}

use kora::pipeline::context;
use tempfile::TempDir;

#[test]
fn test_build_validator_prompt_includes_plan_and_results() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path();

    let context_dir = run_dir.join("context");
    std::fs::create_dir_all(&context_dir).unwrap();
    std::fs::write(
        context_dir.join("researcher-plan.md"),
        "the implementation plan",
    )
    .unwrap();
    std::fs::write(
        context_dir.join("codebase-summary.md"),
        "the codebase summary",
    )
    .unwrap();

    let plan_dir = run_dir.join("plan");
    std::fs::create_dir_all(&plan_dir).unwrap();
    std::fs::write(plan_dir.join("task-breakdown.json"), "{}").unwrap();
    std::fs::write(plan_dir.join("test-strategy.json"), "{}").unwrap();

    let task_dir = run_dir.join("implementation").join("task-T1");
    std::fs::create_dir_all(&task_dir).unwrap();
    std::fs::write(task_dir.join("TASK_RESULT.md"), "## Status: COMPLETE").unwrap();

    let result = context::build_validator_prompt(run_dir, "test request", run_dir, None).unwrap();

    assert!(result.prompt.contains("the implementation plan"));
    assert!(result.prompt.contains("the codebase summary"));
    assert!(result.prompt.contains("Status: COMPLETE"));
    assert!(result.prompt.contains("test request"));
}

#[test]
fn test_build_validator_prompt_no_task_results() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path();

    let context_dir = run_dir.join("context");
    std::fs::create_dir_all(&context_dir).unwrap();
    std::fs::write(context_dir.join("researcher-plan.md"), "plan").unwrap();

    let result = context::build_validator_prompt(run_dir, "request", run_dir, None).unwrap();
    assert!(result.prompt.contains("plan"));
}

#[test]
fn test_build_fix_prompt_includes_fixes() {
    let tmp = TempDir::new().unwrap();
    let run_dir = tmp.path();

    let context_dir = run_dir.join("context");
    std::fs::create_dir_all(&context_dir).unwrap();
    std::fs::write(context_dir.join("researcher-plan.md"), "the plan").unwrap();

    let fixes = vec![
        "Fix 1: Missing export in auth.ts".to_string(),
        "Fix 2: Wrong return type in api.ts".to_string(),
    ];

    let result = context::build_fix_prompt(run_dir, "test request", &fixes, run_dir, None).unwrap();

    assert!(result.prompt.contains("Missing export"));
    assert!(result.prompt.contains("Wrong return type"));
    assert!(result.prompt.contains("Fix Mode"));
}
