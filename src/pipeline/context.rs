use anyhow::Result;
use std::path::Path;

use crate::agent::output_parser::Task;
use crate::agent::prompts;

pub struct PromptContext {
    pub prompt: String,
}

fn read_file_if_exists(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn load_custom_instructions(project_root: &Path, path: Option<&Path>) -> Option<String> {
    let relative = path?;
    let full_path = project_root.join(relative);
    read_file_if_exists(&full_path)
}

pub fn build_researcher_prompt(
    request: &str,
    custom_instructions: Option<&str>,
) -> Result<PromptContext> {
    let base = prompts::RESEARCHER_PROMPT;
    let context = format!("## User Request\n\n{}", request);
    let prompt = prompts::assemble_prompt(base, custom_instructions, &context);
    Ok(PromptContext { prompt })
}

pub fn build_researcher_revision_prompt(
    run_dir: &Path,
    iteration: u32,
    project_root: &Path,
    custom_instructions_path: Option<&Path>,
) -> Result<PromptContext> {
    let base = prompts::RESEARCHER_PROMPT;
    let custom = load_custom_instructions(project_root, custom_instructions_path);

    let plan = read_file_if_exists(&run_dir.join("context").join("researcher-plan.md"))
        .unwrap_or_default();
    let judgment = read_file_if_exists(
        &run_dir
            .join("reviews")
            .join(format!("iteration-{}", iteration))
            .join("judgment.md"),
    )
    .unwrap_or_default();

    let context = format!(
        "## Revision Mode\n\n\
         You are revising your previous plan based on review findings.\n\n\
         ## Current Plan\n\n{}\n\n\
         ## Judge's Findings (valid only)\n\n{}",
        plan, judgment
    );

    let prompt = prompts::assemble_prompt(base, custom.as_deref(), &context);
    Ok(PromptContext { prompt })
}

pub fn build_reviewer_prompt(
    run_dir: &Path,
    iteration: u32,
    request: &str,
    project_root: &Path,
    custom_instructions_path: Option<&Path>,
) -> Result<PromptContext> {
    let base = prompts::REVIEWER_PROMPT;
    let custom = load_custom_instructions(project_root, custom_instructions_path);

    let plan = read_file_if_exists(&run_dir.join("context").join("researcher-plan.md"))
        .unwrap_or_default();
    let codebase_summary = read_file_if_exists(&run_dir.join("context").join("codebase-summary.md"))
        .unwrap_or_default();

    let mut context = format!(
        "## User Request\n\n{}\n\n\
         ## Codebase Summary\n\n{}\n\n\
         ## Implementation Plan\n\n{}",
        request, codebase_summary, plan
    );

    if iteration > 1 {
        for i in 1..iteration {
            let iter_dir = run_dir.join("reviews").join(format!("iteration-{}", i));
            if let Some(prev_review) = read_file_if_exists(&iter_dir.join("review.md")) {
                context.push_str(&format!(
                    "\n\n## Previous Review (Iteration {})\n\n{}",
                    i, prev_review
                ));
            }
            if let Some(prev_judgment) = read_file_if_exists(&iter_dir.join("judgment.md")) {
                context.push_str(&format!(
                    "\n\n## Previous Judgment (Iteration {})\n\n{}",
                    i, prev_judgment
                ));
            }
        }
    }

    let prompt = prompts::assemble_prompt(base, custom.as_deref(), &context);
    Ok(PromptContext { prompt })
}

pub fn build_security_prompt(
    run_dir: &Path,
    iteration: u32,
    request: &str,
    project_root: &Path,
    custom_instructions_path: Option<&Path>,
) -> Result<PromptContext> {
    let base = prompts::SECURITY_AUDITOR_PROMPT;
    let custom = load_custom_instructions(project_root, custom_instructions_path);

    let plan = read_file_if_exists(&run_dir.join("context").join("researcher-plan.md"))
        .unwrap_or_default();
    let codebase_summary = read_file_if_exists(&run_dir.join("context").join("codebase-summary.md"))
        .unwrap_or_default();

    let mut context = format!(
        "## User Request\n\n{}\n\n\
         ## Codebase Summary\n\n{}\n\n\
         ## Implementation Plan\n\n{}",
        request, codebase_summary, plan
    );

    if iteration > 1 {
        for i in 1..iteration {
            let iter_dir = run_dir.join("reviews").join(format!("iteration-{}", i));
            if let Some(prev_audit) = read_file_if_exists(&iter_dir.join("security-audit.md")) {
                context.push_str(&format!(
                    "\n\n## Previous Security Audit (Iteration {})\n\n{}",
                    i, prev_audit
                ));
            }
            if let Some(prev_judgment) = read_file_if_exists(&iter_dir.join("judgment.md")) {
                context.push_str(&format!(
                    "\n\n## Previous Judgment (Iteration {})\n\n{}",
                    i, prev_judgment
                ));
            }
        }
    }

    let prompt = prompts::assemble_prompt(base, custom.as_deref(), &context);
    Ok(PromptContext { prompt })
}

pub fn build_judge_prompt(
    run_dir: &Path,
    iteration: u32,
    request: &str,
    project_root: &Path,
    custom_instructions_path: Option<&Path>,
) -> Result<PromptContext> {
    let base = prompts::JUDGE_PROMPT;
    let custom = load_custom_instructions(project_root, custom_instructions_path);

    let plan = read_file_if_exists(&run_dir.join("context").join("researcher-plan.md"))
        .unwrap_or_default();

    let iter_dir = run_dir
        .join("reviews")
        .join(format!("iteration-{}", iteration));
    let review = read_file_if_exists(&iter_dir.join("review.md")).unwrap_or_default();
    let security = read_file_if_exists(&iter_dir.join("security-audit.md")).unwrap_or_default();

    let mut context = format!(
        "## User Request\n\n{}\n\n\
         ## Implementation Plan\n\n{}\n\n\
         ## Reviewer Findings\n\n{}\n\n\
         ## Security Auditor Findings\n\n{}",
        request, plan, review, security
    );

    if iteration > 1 {
        for i in 1..iteration {
            let prev_dir = run_dir.join("reviews").join(format!("iteration-{}", i));
            if let Some(prev_judgment) = read_file_if_exists(&prev_dir.join("judgment.md")) {
                context.push_str(&format!(
                    "\n\n## Previous Judgment (Iteration {})\n\n{}",
                    i, prev_judgment
                ));
            }
        }
    }

    let prompt = prompts::assemble_prompt(base, custom.as_deref(), &context);
    Ok(PromptContext { prompt })
}

pub fn build_planner_prompt(
    run_dir: &Path,
    request: &str,
    project_root: &Path,
    custom_instructions_path: Option<&Path>,
) -> Result<PromptContext> {
    let base = prompts::PLANNER_PROMPT;
    let custom = load_custom_instructions(project_root, custom_instructions_path);

    let plan = read_file_if_exists(&run_dir.join("context").join("researcher-plan.md"))
        .unwrap_or_default();
    let codebase_summary =
        read_file_if_exists(&run_dir.join("context").join("codebase-summary.md"))
            .unwrap_or_default();

    let context = format!(
        "## User Request\n\n{}\n\n\
         ## Codebase Summary\n\n{}\n\n\
         ## Approved Implementation Plan\n\n{}",
        request, codebase_summary, plan
    );

    let prompt = prompts::assemble_prompt(base, custom.as_deref(), &context);
    Ok(PromptContext { prompt })
}

pub fn build_test_architect_prompt(
    run_dir: &Path,
    request: &str,
    project_root: &Path,
    custom_instructions_path: Option<&Path>,
) -> Result<PromptContext> {
    let base = prompts::TEST_ARCHITECT_PROMPT;
    let custom = load_custom_instructions(project_root, custom_instructions_path);

    let plan = read_file_if_exists(&run_dir.join("context").join("researcher-plan.md"))
        .unwrap_or_default();
    let codebase_summary =
        read_file_if_exists(&run_dir.join("context").join("codebase-summary.md"))
            .unwrap_or_default();
    let task_breakdown =
        read_file_if_exists(&run_dir.join("plan").join("task-breakdown.json")).unwrap_or_default();

    let context = format!(
        "## User Request\n\n{}\n\n\
         ## Codebase Summary\n\n{}\n\n\
         ## Approved Implementation Plan\n\n{}\n\n\
         ## Task Breakdown\n\n{}",
        request, codebase_summary, plan, task_breakdown
    );

    let prompt = prompts::assemble_prompt(base, custom.as_deref(), &context);
    Ok(PromptContext { prompt })
}

pub fn build_implementor_prompt(task: &Task, test_spec: &str) -> Result<String> {
    let base = prompts::IMPLEMENTOR_PROMPT;

    let task_section = format!(
        "## Your Task\n\n\
         **ID:** {}\n\
         **Title:** {}\n\n\
         {}\n\n\
         **Files to create:** {}\n\
         **Files to modify:** {}\n\
         **Files to delete:** {}",
        task.id,
        task.title,
        task.description,
        if task.files.create.is_empty() {
            "none".to_string()
        } else {
            task.files.create.join(", ")
        },
        if task.files.modify.is_empty() {
            "none".to_string()
        } else {
            task.files.modify.join(", ")
        },
        if task.files.delete.is_empty() {
            "none".to_string()
        } else {
            task.files.delete.join(", ")
        },
    );

    let test_section = if test_spec.is_empty() {
        "## Test Requirements\n\nNo specific test requirements for this task.".to_string()
    } else {
        format!("## Test Requirements\n\n{}", test_spec)
    };

    Ok(format!(
        "{}\n\n---\n\n{}\n\n---\n\n{}",
        base, task_section, test_section
    ))
}
