use kora::pipeline::stall::{is_cycling, text_similarity, DEFAULT_CYCLE_THRESHOLD};

#[test]
fn test_identical_texts_similarity_is_one() {
    let text = "line one\nline two\nline three";
    assert!((text_similarity(text, text) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_completely_different_texts_similarity_is_zero() {
    let a = "alpha\nbeta\ngamma";
    let b = "delta\nepsilon\nzeta";
    assert!((text_similarity(a, b)).abs() < f64::EPSILON);
}

#[test]
fn test_partial_overlap_similarity() {
    let a = "shared line\nunique a\nanother shared";
    let b = "shared line\nunique b\nanother shared";
    let sim = text_similarity(a, b);
    assert!(sim > 0.0, "similarity should be above 0");
    assert!(sim < 1.0, "similarity should be below 1");
    // 2 shared out of 4 unique lines total → 0.5
    assert!((sim - 0.5).abs() < f64::EPSILON);
}

#[test]
fn test_empty_texts_similarity_is_one() {
    assert!((text_similarity("", "") - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_one_empty_one_not_similarity_is_zero() {
    assert!((text_similarity("", "some content")).abs() < f64::EPSILON);
    assert!((text_similarity("some content", "")).abs() < f64::EPSILON);
}

#[test]
fn test_whitespace_normalization() {
    let a = "  line one  \n\tline two\t\n   line three   ";
    let b = "line one\nline two\nline three";
    assert!((text_similarity(a, b) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_is_cycling_above_threshold() {
    let a = "finding 1\nfinding 2\nfinding 3\nfinding 4\nfinding 5\nfinding 6\nfinding 7\nfinding 8\nfinding 9\nfinding 10\nfinding 11\nfinding 12\nfinding 13\nfinding 14";
    let b = "finding 1\nfinding 2\nfinding 3\nfinding 4\nfinding 5\nfinding 6\nfinding 7\nfinding 8\nfinding 9\nfinding 10\nfinding 11\nfinding 12\nfinding 13\nfinding NEW";
    assert!(is_cycling(a, b, DEFAULT_CYCLE_THRESHOLD));
}

#[test]
fn test_is_cycling_below_threshold() {
    let a = "alpha\nbeta\ngamma";
    let b = "delta\nepsilon\nzeta";
    assert!(!is_cycling(a, b, DEFAULT_CYCLE_THRESHOLD));
}

#[test]
fn test_is_cycling_with_custom_threshold() {
    let a = "shared\nunique a";
    let b = "shared\nunique b";
    // similarity is 1/3 ≈ 0.333
    assert!(is_cycling(a, b, 0.3));
    assert!(!is_cycling(a, b, 0.5));
}

#[test]
fn test_similarity_ignores_empty_lines() {
    let a = "line one\n\n\nline two\n\n";
    let b = "line one\nline two";
    assert!((text_similarity(a, b) - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_real_world_review_cycling() {
    let review_1 = r#"## Code Review Findings

### Issue 1: Missing error handling in parse_config
The function unwraps without checking for None values.
Severity: High

### Issue 2: Unused import on line 15
The `std::fmt` import is not used anywhere.
Severity: Low

### Issue 3: Variable naming
The variable `x` should have a more descriptive name.
Severity: Medium

<!-- VERDICT -->NEEDS_CHANGES<!-- /VERDICT -->"#;

    let review_2 = r#"## Code Review Findings

### Issue 1: Missing error handling in parse_config
The function uses unwrap without proper None checks.
Severity: High

### Issue 2: Unused import on line 15
The import of `std::fmt` is unnecessary.
Severity: Low

### Issue 3: Variable naming
The variable `x` needs a more descriptive name.
Severity: Medium

<!-- VERDICT -->NEEDS_CHANGES<!-- /VERDICT -->"#;

    let sim = text_similarity(review_1, review_2);
    // Many lines are shared (headings, severity lines, verdict), so similarity should be high
    assert!(
        sim > 0.5,
        "real-world cycling reviews should have high similarity, got {sim}"
    );
    assert!(is_cycling(review_1, review_2, 0.5));
}
