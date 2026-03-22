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

#[allow(clippy::too_many_arguments)]
pub async fn run_merge_flow(
    worktree_manager: &WorktreeManager,
    task_states: &HashMap<String, TaskState>,
    merge_order: &[String],
    run_id: &str,
    config: &Config,
    renderer: &mut Renderer,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    auto_merge: bool,
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

    let choice = if auto_merge {
        renderer.info("auto-merging all branches into current branch");
        0
    } else {
        let options = [
            "Merge all into current branch",
            "Create a single combined branch",
            "Leave branches as-is",
        ];
        selector::select("What would you like to do with the changes?", &options, 0)?
    };

    let strategy = match choice {
        0 => MergeStrategy::MergeIntoCurrent,
        1 => MergeStrategy::CombinedBranch,
        _ => MergeStrategy::LeaveAsIs,
    };

    let outcome = match strategy {
        MergeStrategy::LeaveAsIs => {
            renderer.info("branches left as-is");
            MergeOutcome {
                strategy: MergeStrategy::LeaveAsIs,
                merged_branches: vec![],
                failed_branches: vec![],
            }
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

            MergeOutcome {
                strategy: MergeStrategy::MergeIntoCurrent,
                merged_branches: merged,
                failed_branches: failed,
            }
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

            MergeOutcome {
                strategy: MergeStrategy::CombinedBranch,
                merged_branches: merged,
                failed_branches: failed,
            }
        }
    };

    if !outcome.merged_branches.is_empty() {
        offer_remote_operations(worktree_manager, &outcome, run_id, renderer).await?;
    }

    Ok(outcome)
}

async fn offer_remote_operations(
    worktree_manager: &WorktreeManager,
    _outcome: &MergeOutcome,
    run_id: &str,
    renderer: &mut Renderer,
) -> Result<()> {
    renderer.text("");

    let has_gh = which::which("gh").is_ok();
    let has_remote = check_remote_exists(worktree_manager.repo_root()).await;

    if !has_remote {
        return Ok(());
    }

    let mut options: Vec<&str> = vec!["Done — keep changes local"];

    if has_remote {
        options.push("Push branch to remote");
    }
    if has_gh && has_remote {
        options.push("Push and create a Pull Request");
    }

    let choice = selector::select("Push to remote?", &options, 0)?;

    match choice {
        0 => {
            renderer.info("changes kept local");
        }
        1 => {
            let branch = get_current_branch(worktree_manager.repo_root()).await?;
            push_branch(worktree_manager.repo_root(), &branch, renderer).await?;
        }
        2 if has_gh => {
            let branch = get_current_branch(worktree_manager.repo_root()).await?;
            push_branch(worktree_manager.repo_root(), &branch, renderer).await?;
            create_pull_request(worktree_manager.repo_root(), &branch, run_id, renderer).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn check_remote_exists(repo_root: &Path) -> bool {
    let output = tokio::process::Command::new("git")
        .current_dir(repo_root)
        .args(["remote"])
        .output()
        .await;

    match output {
        Ok(o) => !String::from_utf8_lossy(&o.stdout).trim().is_empty(),
        Err(_) => false,
    }
}

async fn get_current_branch(repo_root: &Path) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .current_dir(repo_root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .await
        .context("failed to get current branch")?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn push_branch(repo_root: &Path, branch: &str, renderer: &mut Renderer) -> Result<()> {
    renderer.info(&format!("pushing {} to remote...", branch));

    let output = tokio::process::Command::new("git")
        .current_dir(repo_root)
        .args(["push", "-u", "origin", branch])
        .output()
        .await
        .context("failed to push branch")?;

    if output.status.success() {
        renderer.stage_complete(&format!("pushed {}", branch), 0);
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        renderer.info(&format!("push failed: {}", stderr.trim()));
    }

    Ok(())
}

async fn create_pull_request(
    repo_root: &Path,
    branch: &str,
    run_id: &str,
    renderer: &mut Renderer,
) -> Result<()> {
    renderer.info("creating pull request...");

    let default_branch = get_default_remote_branch(repo_root).await?;

    let title = format!("kora: {}", run_id);
    let body = format!(
        "## Changes\n\nAutomated by [Kora](https://github.com/usekora/kora) run `{}`.\n\n\
         Review the changes and merge when ready.",
        run_id
    );

    let output = tokio::process::Command::new("gh")
        .current_dir(repo_root)
        .args([
            "pr",
            "create",
            "--base",
            &default_branch,
            "--head",
            branch,
            "--title",
            &title,
            "--body",
            &body,
        ])
        .output()
        .await
        .context("failed to create pull request")?;

    if output.status.success() {
        let pr_url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        renderer.stage_complete("pull request created", 0);
        renderer.text(&format!("  {}", pr_url));
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        renderer.info(&format!("PR creation failed: {}", stderr.trim()));
    }

    Ok(())
}

async fn get_default_remote_branch(repo_root: &Path) -> Result<String> {
    let output = tokio::process::Command::new("git")
        .current_dir(repo_root)
        .args(["symbolic-ref", "refs/remotes/origin/HEAD", "--short"])
        .output()
        .await;

    if let Ok(o) = output {
        if o.status.success() {
            let branch = String::from_utf8_lossy(&o.stdout).trim().to_string();
            return Ok(branch.trim_start_matches("origin/").to_string());
        }
    }

    Ok("main".to_string())
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
