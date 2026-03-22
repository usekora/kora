use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use crate::config::Config;
use crate::git::worktree::{MergeResult, WorktreeManager};
use crate::pipeline::implementation::TaskState;
use crate::provider::Provider;
use crate::terminal::selector;
use crate::terminal::Renderer;

#[derive(Debug, Clone, PartialEq)]
pub enum MergeStrategy {
    MergeIntoCurrent,
    CombinedBranch,
    LeaveAsIs,
}

pub struct MergeOutcome {
    pub strategy: MergeStrategy,
    pub merged_branches: Vec<String>,
    pub failed_branches: Vec<String>,
}

pub async fn run_merge_flow(
    worktree_manager: &WorktreeManager,
    task_states: &HashMap<String, TaskState>,
    merge_order: &[String],
    run_id: &str,
    config: &Config,
    renderer: &mut Renderer,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
) -> Result<MergeOutcome> {
    let branch_list: Vec<String> = merge_order
        .iter()
        .filter_map(|id| task_states.get(id))
        .map(|s| s.branch_name.clone())
        .collect();

    renderer.separator();
    renderer.text("implementation and validation complete.");
    renderer.text("");
    renderer.text("task branches:");
    for (id, state) in merge_order
        .iter()
        .filter_map(|id| task_states.get(id).map(|s| (id, s)))
    {
        renderer.text(&format!("  {} -> {}", id, state.branch_name));
    }
    renderer.text("");

    let options = [
        "Merge all into current branch",
        "Create a single combined branch",
        "Leave branches as-is",
    ];

    let choice = selector::select("What would you like to do with the changes?", &options)?;

    let strategy = match choice {
        0 => MergeStrategy::MergeIntoCurrent,
        1 => MergeStrategy::CombinedBranch,
        _ => MergeStrategy::LeaveAsIs,
    };

    match strategy {
        MergeStrategy::LeaveAsIs => {
            renderer.info("branches left as-is");
            Ok(MergeOutcome {
                strategy: MergeStrategy::LeaveAsIs,
                merged_branches: vec![],
                failed_branches: vec![],
            })
        }
        MergeStrategy::MergeIntoCurrent => {
            let repo_root = worktree_manager.repo_root();
            let (merged, failed) = merge_branches_sequentially(
                worktree_manager,
                repo_root,
                &branch_list,
                merge_order,
                config,
                renderer,
                get_provider,
            )
            .await?;

            Ok(MergeOutcome {
                strategy: MergeStrategy::MergeIntoCurrent,
                merged_branches: merged,
                failed_branches: failed,
            })
        }
        MergeStrategy::CombinedBranch => {
            let combined_branch = format!("kora/combined-{}", run_id);

            let output = tokio::process::Command::new("git")
                .current_dir(worktree_manager.repo_root())
                .arg("checkout")
                .arg("-b")
                .arg(&combined_branch)
                .output()
                .await
                .context("failed to create combined branch")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("failed to create combined branch: {}", stderr);
            }

            let repo_root = worktree_manager.repo_root().to_path_buf();
            let (merged, failed) = merge_branches_sequentially(
                worktree_manager,
                &repo_root,
                &branch_list,
                merge_order,
                config,
                renderer,
                get_provider,
            )
            .await?;

            renderer.info(&format!("combined branch: {}", combined_branch));

            Ok(MergeOutcome {
                strategy: MergeStrategy::CombinedBranch,
                merged_branches: merged,
                failed_branches: failed,
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn merge_branches_sequentially(
    worktree_manager: &WorktreeManager,
    target_dir: &Path,
    branch_list: &[String],
    merge_order: &[String],
    config: &Config,
    renderer: &mut Renderer,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
) -> Result<(Vec<String>, Vec<String>)> {
    let mut merged = Vec::new();
    let mut failed = Vec::new();

    for (i, branch) in branch_list.iter().enumerate() {
        let task_id = merge_order.get(i).map(|s| s.as_str()).unwrap_or("unknown");
        renderer.info(&format!("merging {} ({})", branch, task_id));

        let result = worktree_manager.merge_branch(target_dir, branch).await?;

        match result {
            MergeResult::Success => {
                renderer.stage_complete(&format!("merged {}", branch), 0);
                merged.push(branch.clone());
            }
            MergeResult::Conflict { files } => {
                renderer.escalation(&format!(
                    "conflict merging {}: {}",
                    branch,
                    files.join(", ")
                ));

                let resolved = attempt_conflict_resolution(
                    worktree_manager,
                    target_dir,
                    branch,
                    task_id,
                    &files,
                    config,
                    get_provider,
                    renderer,
                )
                .await?;

                if resolved {
                    merged.push(branch.clone());
                } else {
                    renderer.info(&format!(
                        "skipping branch {} due to unresolved conflict",
                        branch
                    ));
                    failed.push(branch.clone());
                }
            }
        }
    }

    Ok((merged, failed))
}

#[allow(clippy::too_many_arguments)]
async fn attempt_conflict_resolution(
    _worktree_manager: &WorktreeManager,
    target_dir: &Path,
    branch: &str,
    task_id: &str,
    conflict_files: &[String],
    config: &Config,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    renderer: &mut Renderer,
) -> Result<bool> {
    let provider = match get_provider(&config.agents.implementor.provider) {
        Some(p) => p,
        None => return Ok(false),
    };

    let conflict_context = conflict_files.join("\n- ");
    let prompt = format!(
        "You are resolving a git merge conflict.\n\n\
         Branch being merged: {}\n\
         Task: {}\n\n\
         Conflicting files:\n- {}\n\n\
         The merge has been started but has conflicts. \
         Resolve all conflicts in the working directory, \
         then run `git add` on the resolved files and `git commit --no-edit` to complete the merge.\n\n\
         Ensure all tests still pass after resolution.",
        branch, task_id, conflict_context
    );

    let merge_output = tokio::process::Command::new("git")
        .current_dir(target_dir)
        .arg("merge")
        .arg(branch)
        .arg("--no-edit")
        .output()
        .await
        .context("failed to start merge for conflict resolution")?;

    if merge_output.status.success() {
        return Ok(true);
    }

    renderer.info(&format!(
        "spawning agent to resolve conflict for {}",
        task_id
    ));

    let no_flags: Vec<String> = vec![];
    let result = provider
        .run(
            &prompt,
            target_dir,
            &no_flags,
            Some(Duration::from_secs(300)),
        )
        .await;

    match result {
        Ok(output) => {
            if output.exit_code == 0 {
                let status = tokio::process::Command::new("git")
                    .current_dir(target_dir)
                    .arg("diff")
                    .arg("--name-only")
                    .arg("--diff-filter=U")
                    .output()
                    .await?;

                let remaining_conflicts = String::from_utf8_lossy(&status.stdout);
                if remaining_conflicts.trim().is_empty() {
                    renderer.stage_complete(&format!("conflict resolved for {}", task_id), 0);
                    return Ok(true);
                }
            }

            let _ = tokio::process::Command::new("git")
                .current_dir(target_dir)
                .arg("merge")
                .arg("--abort")
                .output()
                .await;

            Ok(false)
        }
        Err(_) => {
            let _ = tokio::process::Command::new("git")
                .current_dir(target_dir)
                .arg("merge")
                .arg("--abort")
                .output()
                .await;
            Ok(false)
        }
    }
}
