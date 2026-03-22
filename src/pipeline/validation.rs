use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::agent::output_parser::{self, ValidationResult};
use crate::provider::Provider;
use crate::state::RunDirectory;

pub struct ValidatorOutput {
    pub validation: ValidationResult,
    pub report_path: PathBuf,
    pub raw_text: String,
}

pub async fn run_validator(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &RunDirectory,
    extra_flags: &[String],
) -> Result<ValidatorOutput> {
    let output = provider
        .run(prompt, working_dir, extra_flags)
        .await
        .context("validator agent failed")?;

    if output.exit_code != 0 {
        anyhow::bail!("validator exited with code {}", output.exit_code);
    }

    let validation_dir = run_dir.validation_dir();
    std::fs::create_dir_all(&validation_dir)?;

    let report_path = validation_dir.join("report.md");
    std::fs::write(&report_path, &output.text)?;

    let validation = output_parser::parse_validation(&output.text).context(
        "validator output missing structured <!-- VALIDATION --> markers",
    )?;

    let status_json = serde_json::json!({
        "status": if validation.passed { "PASS" } else { "FAIL" },
        "blocking_issues": validation.blocking_issues,
        "minor_issues": validation.minor_issues,
        "tests_passed": validation.tests_passed,
        "tests_failed": validation.tests_failed,
        "type_check_passed": validation.type_check_passed,
    });
    std::fs::write(
        validation_dir.join("status.json"),
        serde_json::to_string_pretty(&status_json)?,
    )?;

    Ok(ValidatorOutput {
        validation,
        report_path,
        raw_text: output.text,
    })
}

pub fn extract_required_fixes(report_text: &str) -> Vec<String> {
    let mut fixes = Vec::new();
    let mut current_fix = String::new();
    let mut in_fixes = false;

    for line in report_text.lines() {
        if line.trim() == "## Required Fixes" {
            in_fixes = true;
            continue;
        }
        if in_fixes {
            if line.starts_with("## ") && !line.starts_with("### Fix") {
                if !current_fix.trim().is_empty() {
                    fixes.push(current_fix.trim().to_string());
                }
                break;
            }
            if line.starts_with("### Fix") {
                if !current_fix.trim().is_empty() {
                    fixes.push(current_fix.trim().to_string());
                }
                current_fix = String::new();
            }
            current_fix.push_str(line);
            current_fix.push('\n');
        }
    }

    if in_fixes && !current_fix.trim().is_empty() {
        fixes.push(current_fix.trim().to_string());
    }

    fixes
}
