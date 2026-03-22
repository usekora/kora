use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::Path;

use crate::agent::output_parser::{self, extract_json_object, TaskBreakdown};
use crate::provider::Provider;
use crate::state::RunDirectory;

pub async fn run_planner(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &RunDirectory,
    extra_flags: &[String],
) -> Result<TaskBreakdown> {
    let output = provider
        .run(prompt, working_dir, extra_flags)
        .await
        .context("planner agent failed")?;

    if output.exit_code != 0 {
        anyhow::bail!("planner exited with code {}", output.exit_code);
    }

    let plan_dir = run_dir.plan_dir();
    std::fs::create_dir_all(&plan_dir)?;
    std::fs::write(plan_dir.join("planner-output.md"), &output.text)?;

    let json_text =
        extract_json_object(&output.text).context("planner output does not contain valid JSON")?;

    let breakdown: TaskBreakdown = output_parser::parse_task_breakdown(&json_text)
        .context("failed to parse task breakdown JSON")?;

    validate_breakdown(&breakdown)?;

    std::fs::write(
        plan_dir.join("task-breakdown.json"),
        serde_json::to_string_pretty(&breakdown)?,
    )?;

    Ok(breakdown)
}

pub fn validate_breakdown(breakdown: &TaskBreakdown) -> Result<()> {
    let ids: HashSet<&str> = breakdown.tasks.iter().map(|t| t.id.as_str()).collect();

    if ids.len() != breakdown.tasks.len() {
        anyhow::bail!("task breakdown contains duplicate task IDs");
    }

    for task in &breakdown.tasks {
        for dep in &task.depends_on {
            if !ids.contains(dep.as_str()) {
                anyhow::bail!("task {} depends on non-existent task {}", task.id, dep);
            }
        }
    }

    for task in &breakdown.tasks {
        if task.depends_on.contains(&task.id) {
            anyhow::bail!("task {} depends on itself", task.id);
        }
    }

    for id in &breakdown.merge_order {
        if !ids.contains(id.as_str()) {
            anyhow::bail!("merge_order references non-existent task {}", id);
        }
    }

    Ok(())
}
