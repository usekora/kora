use anyhow::Result;
use std::path::Path;

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
