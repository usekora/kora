use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

pub struct WorktreeManager {
    repo_root: PathBuf,
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MergeResult {
    Success,
    Conflict { files: Vec<String> },
}

const WORKTREE_PREFIX: &str = "kora-worktree-";

impl WorktreeManager {
    pub fn new(repo_root: &Path) -> Self {
        Self {
            repo_root: repo_root.to_path_buf(),
        }
    }

    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    pub async fn create_worktree(&self, task_id: &str, branch_name: &str) -> Result<PathBuf> {
        let worktree_dir = self
            .repo_root
            .parent()
            .unwrap_or(&self.repo_root)
            .join(format!("{}{}", WORKTREE_PREFIX, task_id.to_lowercase()));

        let output = Command::new("git")
            .current_dir(&self.repo_root)
            .arg("worktree")
            .arg("add")
            .arg(&worktree_dir)
            .arg("-b")
            .arg(branch_name)
            .output()
            .await
            .context("failed to run git worktree add")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git worktree add failed: {}", stderr);
        }

        Ok(worktree_dir)
    }

    pub async fn merge_dependency_branches(
        &self,
        worktree_path: &Path,
        dependency_branches: &[String],
    ) -> Result<MergeResult> {
        for branch in dependency_branches {
            let output = Command::new("git")
                .current_dir(worktree_path)
                .arg("merge")
                .arg(branch)
                .arg("--no-edit")
                .output()
                .await
                .context("failed to run git merge")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let conflict_files = self.list_conflict_files(worktree_path).await;
                let _ = Command::new("git")
                    .current_dir(worktree_path)
                    .arg("merge")
                    .arg("--abort")
                    .output()
                    .await;
                return Ok(MergeResult::Conflict {
                    files: conflict_files.unwrap_or_else(|_| vec![stderr.to_string()]),
                });
            }
        }
        Ok(MergeResult::Success)
    }

    pub async fn remove_worktree(&self, worktree_path: &Path) -> Result<()> {
        let output = Command::new("git")
            .current_dir(&self.repo_root)
            .arg("worktree")
            .arg("remove")
            .arg(worktree_path)
            .arg("--force")
            .output()
            .await
            .context("failed to run git worktree remove")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git worktree remove failed: {}", stderr);
        }

        Ok(())
    }

    pub async fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        let output = Command::new("git")
            .current_dir(&self.repo_root)
            .arg("worktree")
            .arg("list")
            .arg("--porcelain")
            .output()
            .await
            .context("failed to run git worktree list")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut worktrees = Vec::new();
        let mut current_path: Option<PathBuf> = None;
        let mut current_branch = String::new();

        for line in stdout.lines() {
            if let Some(path_str) = line.strip_prefix("worktree ") {
                current_path = Some(PathBuf::from(path_str));
            } else if let Some(branch_ref) = line.strip_prefix("branch ") {
                current_branch = branch_ref
                    .rsplit('/')
                    .next()
                    .unwrap_or(branch_ref)
                    .to_string();
            } else if line.is_empty() {
                if let Some(path) = current_path.take() {
                    let file_name = path
                        .file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let task_id = file_name
                        .strip_prefix(WORKTREE_PREFIX)
                        .map(|s| s.to_string());
                    worktrees.push(WorktreeInfo {
                        path,
                        branch: std::mem::take(&mut current_branch),
                        task_id,
                    });
                }
            }
        }

        if let Some(path) = current_path.take() {
            let file_name = path
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            let task_id = file_name
                .strip_prefix(WORKTREE_PREFIX)
                .map(|s| s.to_string());
            worktrees.push(WorktreeInfo {
                path,
                branch: current_branch,
                task_id,
            });
        }

        Ok(worktrees)
    }

    pub async fn cleanup_all(&self) -> Result<u32> {
        let worktrees = self.list_worktrees().await?;
        let mut removed = 0u32;

        for wt in worktrees {
            if wt.task_id.is_some() {
                self.remove_worktree(&wt.path).await?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    pub async fn current_branch(&self) -> Result<String> {
        let output = Command::new("git")
            .current_dir(&self.repo_root)
            .arg("rev-parse")
            .arg("--abbrev-ref")
            .arg("HEAD")
            .output()
            .await
            .context("failed to get current branch")?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    pub async fn merge_branch(&self, target_dir: &Path, branch_name: &str) -> Result<MergeResult> {
        let output = Command::new("git")
            .current_dir(target_dir)
            .arg("merge")
            .arg(branch_name)
            .arg("--no-edit")
            .output()
            .await
            .context("failed to run git merge")?;

        if !output.status.success() {
            let conflict_files = self.list_conflict_files(target_dir).await;
            let _ = Command::new("git")
                .current_dir(target_dir)
                .arg("merge")
                .arg("--abort")
                .output()
                .await;
            return Ok(MergeResult::Conflict {
                files: conflict_files.unwrap_or_default(),
            });
        }

        Ok(MergeResult::Success)
    }

    async fn list_conflict_files(&self, dir: &Path) -> Result<Vec<String>> {
        let output = Command::new("git")
            .current_dir(dir)
            .arg("diff")
            .arg("--name-only")
            .arg("--diff-filter=U")
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.lines().map(|l| l.to_string()).collect())
    }
}
