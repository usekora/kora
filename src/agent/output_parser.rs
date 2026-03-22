use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBreakdown {
    pub tasks: Vec<Task>,
    pub branch_strategy: String,
    pub merge_order: Vec<String>,
    pub critical_path: Vec<String>,
    pub parallelism_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub files: TaskFiles,
    pub depends_on: Vec<String>,
    pub estimated_complexity: String,
    #[serde(default)]
    pub conflict_risk: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFiles {
    pub create: Vec<String>,
    pub modify: Vec<String>,
    pub delete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStrategy {
    pub per_task: HashMap<String, TaskTestSpec>,
    pub post_merge: PostMergeTests,
    pub testing_patterns: TestingPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTestSpec {
    pub unit_tests: Vec<TestSpec>,
    pub integration_tests: Vec<TestSpec>,
    pub edge_case_tests: Vec<TestSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSpec {
    pub description: String,
    pub file: String,
    pub setup: String,
    pub expected: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMergeTests {
    pub integration_tests: Vec<PostMergeTestSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMergeTestSpec {
    pub description: String,
    pub tasks_involved: Vec<String>,
    pub setup: String,
    pub expected: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingPatterns {
    pub framework: String,
    pub conventions: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Complete,
    Failed,
    Conflict,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub status: TaskStatus,
    pub changes: Vec<String>,
    pub tests_written: u32,
    pub tests_passing: u32,
    pub tests_failing: u32,
    pub conflicts: Vec<String>,
    pub observations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Verdict {
    pub findings: Vec<FindingVerdict>,
    pub overall: String,
    pub valid_count: u32,
    pub dismissed_count: u32,
}

#[derive(Debug, Clone)]
pub struct FindingVerdict {
    pub id: String,
    pub verdict: String,
}

#[derive(Debug, Clone)]
pub struct ReviewSummary {
    pub findings: Vec<ReviewFinding>,
    pub total: u32,
}

#[derive(Debug, Clone)]
pub struct ReviewFinding {
    pub id: String,
    pub severity: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub passed: bool,
    pub blocking_issues: u32,
    pub minor_issues: u32,
    pub tests_passed: u32,
    pub tests_failed: u32,
    pub type_check_passed: bool,
}

fn extract_block(text: &str, open_tag: &str, close_tag: &str) -> Option<String> {
    let start = text.find(open_tag)?;
    let end = text.find(close_tag)?;
    if end <= start {
        return None;
    }
    let content = &text[start + open_tag.len()..end];
    Some(content.trim().to_string())
}

fn parse_findings_block(block: &str) -> Option<ReviewSummary> {
    let mut findings = Vec::new();
    let mut total = 0u32;

    for line in block.lines() {
        let line = line.trim().trim_start_matches('-').trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with("TOTAL:") {
            if let Some(num) = line.split_whitespace().nth(1) {
                total = num.parse().unwrap_or(0);
            }
        } else if let Some((id_part, rest)) = line.split_once(':') {
            let parts: Vec<&str> = rest.trim().splitn(2, ' ').collect();
            if parts.len() == 2 {
                findings.push(ReviewFinding {
                    id: id_part.trim().to_string(),
                    severity: parts[0].to_string(),
                    title: parts[1].to_string(),
                });
            }
        }
    }

    Some(ReviewSummary {
        total: if total > 0 {
            total
        } else {
            findings.len() as u32
        },
        findings,
    })
}

pub fn parse_verdict(text: &str) -> Option<Verdict> {
    let block = extract_block(text, "<!-- VERDICT -->", "<!-- /VERDICT -->")?;
    let mut findings = Vec::new();
    let mut overall = String::new();
    let mut valid_count = 0u32;
    let mut dismissed_count = 0u32;

    for line in block.lines() {
        let line = line.trim().trim_start_matches('-').trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("OVERALL:") {
            overall = rest.trim().to_string();
        } else if let Some(rest) = line.strip_prefix("VALID_COUNT:") {
            valid_count = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("DISMISSED_COUNT:") {
            dismissed_count = rest.trim().parse().unwrap_or(0);
        } else if let Some((id, verdict)) = line.split_once(':') {
            findings.push(FindingVerdict {
                id: id.trim().to_string(),
                verdict: verdict.trim().to_string(),
            });
        }
    }

    if overall.is_empty() {
        return None;
    }

    Some(Verdict {
        findings,
        overall,
        valid_count,
        dismissed_count,
    })
}

pub fn parse_review(text: &str) -> Option<ReviewSummary> {
    let block = extract_block(text, "<!-- REVIEW -->", "<!-- /REVIEW -->")?;
    parse_findings_block(&block)
}

pub fn parse_security_review(text: &str) -> Option<ReviewSummary> {
    let block = extract_block(text, "<!-- SECURITY -->", "<!-- /SECURITY -->")?;
    parse_findings_block(&block)
}

pub fn parse_code_review(text: &str) -> Option<ReviewSummary> {
    let block = extract_block(text, "<!-- CODE_REVIEW -->", "<!-- /CODE_REVIEW -->")?;
    parse_findings_block(&block)
}

pub fn parse_code_security_review(text: &str) -> Option<ReviewSummary> {
    let block = extract_block(text, "<!-- CODE_SECURITY -->", "<!-- /CODE_SECURITY -->")?;
    parse_findings_block(&block)
}

pub fn extract_plan(text: &str) -> Option<String> {
    extract_block(text, "<!-- PLAN -->", "<!-- /PLAN -->")
}

pub fn parse_validation(text: &str) -> Option<ValidationResult> {
    let block = extract_block(text, "<!-- VALIDATION -->", "<!-- /VALIDATION -->")?;
    let mut passed = false;
    let mut blocking = 0u32;
    let mut minor = 0u32;
    let mut tests_passed = 0u32;
    let mut tests_failed = 0u32;
    let mut type_check = false;

    for line in block.lines() {
        let line = line.trim().trim_start_matches('-').trim();

        if let Some(rest) = line.strip_prefix("STATUS:") {
            passed = rest.trim().eq_ignore_ascii_case("PASS");
        } else if let Some(rest) = line.strip_prefix("BLOCKING_ISSUES:") {
            blocking = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("MINOR_ISSUES:") {
            minor = rest.trim().parse().unwrap_or(0);
        } else if let Some(rest) = line.strip_prefix("TEST_SUITE:") {
            let parts: Vec<&str> = rest.split(',').collect();
            if let Some(p) = parts.first() {
                tests_passed = p
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            }
            if let Some(f) = parts.get(1) {
                tests_failed = f
                    .split_whitespace()
                    .next()
                    .and_then(|n| n.parse().ok())
                    .unwrap_or(0);
            }
        } else if let Some(rest) = line.strip_prefix("TYPE_CHECK:") {
            type_check = rest.trim().eq_ignore_ascii_case("PASS");
        }
    }

    Some(ValidationResult {
        passed,
        blocking_issues: blocking,
        minor_issues: minor,
        tests_passed,
        tests_failed,
        type_check_passed: type_check,
    })
}

pub fn parse_task_breakdown(text: &str) -> Result<TaskBreakdown, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn parse_test_strategy(text: &str) -> Result<TestStrategy, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn extract_json_object(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut best: Option<String> = None;
    let mut search_from = 0;

    while search_from < bytes.len() {
        let start = match text[search_from..].find('{') {
            Some(pos) => search_from + pos,
            None => break,
        };

        if let Some(candidate) = extract_balanced_json(text, start) {
            let is_larger = best.as_ref().is_none_or(|b| candidate.len() > b.len());
            if is_larger {
                best = Some(candidate);
            }
        }

        search_from = start + 1;
    }

    best
}

fn extract_balanced_json(text: &str, start: usize) -> Option<String> {
    let bytes = &text.as_bytes()[start..];
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, &b) in bytes.iter().enumerate() {
        if escape_next {
            escape_next = false;
            continue;
        }
        if b == b'\\' && in_string {
            escape_next = true;
            continue;
        }
        if b == b'"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                let candidate = &text[start..start + i + 1];
                if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                    return Some(candidate.to_string());
                }
                return None;
            }
        }
    }
    None
}

pub fn parse_task_result(text: &str) -> Option<TaskResult> {
    let status = if text.contains("Status: COMPLETE") || text.contains("Status:COMPLETE") {
        TaskStatus::Complete
    } else if text.contains("Status: FAILED") || text.contains("Status:FAILED") {
        TaskStatus::Failed
    } else if text.contains("Status: CONFLICT") || text.contains("Status:CONFLICT") {
        TaskStatus::Conflict
    } else {
        return None;
    };

    let changes = extract_section_items(text, "## Changes Made");
    let conflicts = extract_section_items(text, "## Conflicts");
    let observations = extract_section_items(text, "## Out of Scope Observations");

    let (tests_written, tests_passing, tests_failing) = parse_test_counts(text);

    Some(TaskResult {
        status,
        changes,
        tests_written,
        tests_passing,
        tests_failing,
        conflicts,
        observations,
    })
}

fn extract_section_items(text: &str, header: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut in_section = false;

    for line in text.lines() {
        if line.trim().starts_with(header) {
            in_section = true;
            continue;
        }
        if in_section {
            if line.starts_with("## ") {
                break;
            }
            let trimmed = line.trim().trim_start_matches('-').trim();
            if !trimmed.is_empty() {
                items.push(trimmed.to_string());
            }
        }
    }

    items
}

fn parse_test_counts(text: &str) -> (u32, u32, u32) {
    let mut written = 0u32;
    let mut passing = 0u32;
    let mut failing = 0u32;

    for line in text.lines() {
        let trimmed = line.trim().trim_start_matches('-').trim();
        if trimmed.contains("tests written") || trimmed.contains("test written") {
            let nums: Vec<u32> = trimmed
                .split_whitespace()
                .filter_map(|w| w.parse().ok())
                .collect();
            if nums.len() >= 3 {
                written = nums[0];
                passing = nums[1];
                failing = nums[2];
            } else if nums.len() == 2 {
                written = nums[0];
                passing = nums[1];
            } else if let Some(&n) = nums.first() {
                written = n;
            }
        }
    }

    (written, passing, failing)
}
