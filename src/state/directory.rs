use anyhow::Result;
use std::path::{Path, PathBuf};

use super::RunState;

pub struct RunDirectory {
    base: PathBuf,
}

impl RunDirectory {
    pub fn new(runs_dir: &Path, run_id: &str) -> Self {
        Self {
            base: runs_dir.join(run_id),
        }
    }

    pub fn create_structure(&self) -> Result<()> {
        let dirs = [
            "context",
            "reviews",
            "plan",
            "implementation",
            "validation",
        ];
        for dir in dirs {
            std::fs::create_dir_all(self.base.join(dir))?;
        }
        Ok(())
    }

    pub fn context_dir(&self) -> PathBuf {
        self.base.join("context")
    }

    pub fn reviews_dir(&self, iteration: u32) -> PathBuf {
        self.base.join("reviews").join(format!("iteration-{}", iteration))
    }

    pub fn plan_dir(&self) -> PathBuf {
        self.base.join("plan")
    }

    pub fn task_dir(&self, task_id: &str) -> PathBuf {
        self.base.join("implementation").join(format!("task-{}", task_id))
    }

    pub fn validation_dir(&self) -> PathBuf {
        self.base.join("validation")
    }

    pub fn list_interrupted(runs_dir: &Path) -> Result<Vec<RunState>> {
        let mut interrupted = Vec::new();
        if !runs_dir.exists() {
            return Ok(interrupted);
        }
        for entry in std::fs::read_dir(runs_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                let run_id = entry.file_name().to_string_lossy().to_string();
                if let Ok(state) = RunState::load(runs_dir, &run_id) {
                    match state.status {
                        crate::state::Stage::Complete | crate::state::Stage::Failed(_) => {}
                        _ => interrupted.push(state),
                    }
                }
            }
        }
        interrupted.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(interrupted)
    }
}
