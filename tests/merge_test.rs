use kora::pipeline::merge::MergeStrategy;

#[test]
fn test_merge_strategy_merge_into_current_equality() {
    assert_eq!(
        MergeStrategy::MergeIntoCurrent,
        MergeStrategy::MergeIntoCurrent
    );
}

#[test]
fn test_merge_strategy_combined_branch_equality() {
    assert_eq!(MergeStrategy::CombinedBranch, MergeStrategy::CombinedBranch);
}

#[test]
fn test_merge_strategy_leave_as_is_equality() {
    assert_eq!(MergeStrategy::LeaveAsIs, MergeStrategy::LeaveAsIs);
}

#[test]
fn test_merge_strategy_variants_not_equal() {
    assert_ne!(MergeStrategy::MergeIntoCurrent, MergeStrategy::LeaveAsIs);
    assert_ne!(MergeStrategy::CombinedBranch, MergeStrategy::LeaveAsIs);
    assert_ne!(
        MergeStrategy::MergeIntoCurrent,
        MergeStrategy::CombinedBranch
    );
}
