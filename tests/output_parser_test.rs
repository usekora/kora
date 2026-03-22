use kora::agent::output_parser::{
    extract_plan, parse_review, parse_security_review, parse_validation, parse_verdict,
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
