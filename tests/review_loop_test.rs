use kora::pipeline::review_loop::ReviewOutcome;

#[test]
fn test_review_outcome_approved_equality() {
    assert_eq!(ReviewOutcome::Approved, ReviewOutcome::Approved);
}

#[test]
fn test_review_outcome_escalated_equality() {
    let a = ReviewOutcome::Escalated {
        iteration: 3,
        reason: "did not converge".to_string(),
    };
    let b = ReviewOutcome::Escalated {
        iteration: 3,
        reason: "did not converge".to_string(),
    };
    assert_eq!(a, b);
}

#[test]
fn test_review_outcome_approved_not_equal_escalated() {
    let escalated = ReviewOutcome::Escalated {
        iteration: 1,
        reason: "test".to_string(),
    };
    assert_ne!(ReviewOutcome::Approved, escalated);
}
