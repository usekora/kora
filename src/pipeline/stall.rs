/// Stall detection utilities for detecting cycling agents and stuck loops.
/// Used by code review loops and validation loops to break early when agents
/// produce repetitive output.
use std::collections::HashSet;

/// Default similarity threshold for cycle detection.
pub const DEFAULT_CYCLE_THRESHOLD: f64 = 0.85;

/// Compute similarity ratio between two texts using line-level Jaccard similarity.
/// Returns a value between 0.0 (completely different) and 1.0 (identical).
pub fn text_similarity(a: &str, b: &str) -> f64 {
    let set_a: HashSet<&str> = a
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    let set_b: HashSet<&str> = b
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if set_a.is_empty() && set_b.is_empty() {
        return 1.0;
    }
    if set_a.is_empty() || set_b.is_empty() {
        return 0.0;
    }

    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    intersection as f64 / union as f64
}

/// Returns true if two consecutive outputs indicate a cycling agent.
/// Default threshold is 0.85 (85% similar).
pub fn is_cycling(previous: &str, current: &str, threshold: f64) -> bool {
    text_similarity(previous, current) >= threshold
}
