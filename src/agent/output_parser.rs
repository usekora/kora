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
        total: if total > 0 { total } else { findings.len() as u32 },
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
