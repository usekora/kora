use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::agent::output_parser;
use crate::provider::Provider;
use crate::state::RunDirectory;

pub struct ResearcherResult {
    pub plan_path: PathBuf,
    pub summary_path: Option<PathBuf>,
}

pub async fn run_interactive(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &RunDirectory,
    extra_flags: &[String],
) -> Result<ResearcherResult> {
    let mut session = provider
        .run_interactive(prompt, working_dir, extra_flags)
        .await
        .context("failed to start interactive researcher session")?;

    let status = session.child.wait().await?;
    if !status.success() {
        anyhow::bail!(
            "researcher exited with code {}",
            status.code().unwrap_or(-1)
        );
    }

    let context_dir = run_dir.context_dir();
    std::fs::create_dir_all(&context_dir)?;

    let plan_source = working_dir.join("context").join("researcher-plan.md");
    let plan_dest = context_dir.join("researcher-plan.md");

    if plan_source.exists() {
        std::fs::copy(&plan_source, &plan_dest)?;
    }

    let summary_source = working_dir.join("context").join("codebase-summary.md");
    let summary_dest = context_dir.join("codebase-summary.md");
    let summary_path = if summary_source.exists() {
        std::fs::copy(&summary_source, &summary_dest)?;
        Some(summary_dest)
    } else {
        None
    };

    if !plan_dest.exists() {
        anyhow::bail!("researcher did not produce a plan file at context/researcher-plan.md");
    }

    Ok(ResearcherResult {
        plan_path: plan_dest,
        summary_path,
    })
}

pub async fn run_revision(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &RunDirectory,
    extra_flags: &[String],
    timeout_seconds: u64,
) -> Result<ResearcherResult> {
    let output = provider
        .run(
            prompt,
            working_dir,
            extra_flags,
            Some(Duration::from_secs(timeout_seconds)),
        )
        .await
        .context("failed to run researcher revision")?;

    if output.exit_code != 0 {
        anyhow::bail!("researcher revision exited with code {}", output.exit_code);
    }

    let context_dir = run_dir.context_dir();
    std::fs::create_dir_all(&context_dir)?;

    let plan_dest = context_dir.join("researcher-plan.md");

    let plan_source = working_dir.join("context").join("researcher-plan.md");
    if plan_source.exists() {
        std::fs::copy(&plan_source, &plan_dest)?;
    } else if let Some(plan_text) = output_parser::extract_plan(&output.text) {
        std::fs::write(&plan_dest, &plan_text)?;
    } else {
        anyhow::bail!("researcher revision did not produce a plan (no file or markers found)");
    }

    Ok(ResearcherResult {
        plan_path: plan_dest,
        summary_path: None,
    })
}
