use kora::provider::retry::is_rate_limit_error;

#[test]
fn test_is_rate_limit_429() {
    assert!(is_rate_limit_error("Error 429: too many requests"));
}

#[test]
fn test_is_rate_limit_too_many_requests() {
    assert!(is_rate_limit_error("too many requests"));
}

#[test]
fn test_is_rate_limit_overloaded() {
    assert!(is_rate_limit_error("server is overloaded"));
}

#[test]
fn test_is_rate_limit_rate_limit() {
    assert!(is_rate_limit_error("rate limit exceeded"));
}

#[test]
fn test_is_rate_limit_resource_exhausted() {
    assert!(is_rate_limit_error("resource_exhausted"));
}

#[test]
fn test_is_rate_limit_normal_error() {
    assert!(!is_rate_limit_error("file not found"));
}

#[test]
fn test_is_rate_limit_empty() {
    assert!(!is_rate_limit_error(""));
}

#[test]
fn test_is_rate_limit_case_insensitive() {
    assert!(is_rate_limit_error("Rate Limit"));
}
