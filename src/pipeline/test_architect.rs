use anyhow::{Context, Result};
use std::path::Path;

use crate::agent::output_parser::{self, extract_json_object, TestStrategy};
use crate::provider::Provider;
use crate::state::RunDirectory;

pub async fn run_test_architect(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &RunDirectory,
    extra_flags: &[String],
) -> Result<TestStrategy> {
    let output = provider
        .run(prompt, working_dir, extra_flags)
        .await
        .context("test architect agent failed")?;

    if output.exit_code != 0 {
        anyhow::bail!("test architect exited with code {}", output.exit_code);
    }

    let plan_dir = run_dir.plan_dir();
    std::fs::create_dir_all(&plan_dir)?;
    std::fs::write(plan_dir.join("test-architect-output.md"), &output.text)?;

    let json_text = extract_json_object(&output.text)
        .context("test architect output does not contain valid JSON")?;

    let strategy: TestStrategy = output_parser::parse_test_strategy(&json_text)
        .context("failed to parse test strategy JSON")?;

    std::fs::write(
        plan_dir.join("test-strategy.json"),
        serde_json::to_string_pretty(&strategy)?,
    )?;

    Ok(strategy)
}
