use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context as AnyhowContext, Result};
use tokio::sync::mpsc;

use crate::agent::output_parser::{
    self, Task, TaskBreakdown, TaskResult, TaskStatus, TestStrategy,
};
use crate::config::Config;
use crate::git::worktree::WorktreeManager;
use crate::pipeline::context;
use crate::provider::Provider;
use crate::state::RunDirectory;

#[derive(Debug, Clone, PartialEq)]
pub enum ImplementationTaskStatus {
    Pending,
    Blocked {
        waiting_on: Vec<String>,
    },
    Running {
        worktree_path: PathBuf,
        provider: String,
    },
    Complete {
        duration_secs: u64,
        files_changed: u32,
    },
    Failed {
        error: String,
        attempts: u32,
    },
    Conflict {
        details: String,
    },
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub task: Task,
    pub status: ImplementationTaskStatus,
    pub branch_name: String,
    pub attempts: u32,
}

#[derive(Debug)]
pub enum TaskEvent {
    Started {
        task_id: String,
        provider: String,
        worktree_path: PathBuf,
    },
    Completed {
        task_id: String,
        result: TaskResult,
        duration_secs: u64,
        files_changed: u32,
    },
    Failed {
        task_id: String,
        error: String,
        attempts: u32,
    },
}

pub struct ImplementationFleet {
    config: Config,
    breakdown: TaskBreakdown,
    test_strategy: TestStrategy,
    worktree_manager: WorktreeManager,
    run_dir: PathBuf,
    task_states: HashMap<String, TaskState>,
}

impl ImplementationFleet {
    pub fn new(
        config: Config,
        breakdown: TaskBreakdown,
        test_strategy: TestStrategy,
        project_root: &Path,
        run_dir: &RunDirectory,
    ) -> Self {
        let worktree_manager = WorktreeManager::new(project_root);
        let mut task_states = HashMap::new();

        for task in &breakdown.tasks {
            let waiting_on: Vec<String> = task.depends_on.clone();
            let status = if waiting_on.is_empty() {
                ImplementationTaskStatus::Pending
            } else {
                ImplementationTaskStatus::Blocked { waiting_on }
            };
            let branch_name = format!("kora/{}", task.id.to_lowercase());
            task_states.insert(
                task.id.clone(),
                TaskState {
                    task: task.clone(),
                    status,
                    branch_name,
                    attempts: 0,
                },
            );
        }

        Self {
            config,
            breakdown,
            test_strategy,
            worktree_manager,
            run_dir: run_dir
                .plan_dir()
                .parent()
                .unwrap_or(project_root)
                .to_path_buf(),
            task_states,
        }
    }

    pub fn ready_tasks(&self) -> Vec<String> {
        let mut ready: Vec<String> = self
            .task_states
            .iter()
            .filter(|(_, s)| matches!(s.status, ImplementationTaskStatus::Pending))
            .map(|(id, _)| id.clone())
            .collect();
        ready.sort();
        ready
    }

    pub fn check_unblocked(&mut self) -> Vec<String> {
        let completed: HashSet<String> = self
            .task_states
            .iter()
            .filter(|(_, s)| matches!(s.status, ImplementationTaskStatus::Complete { .. }))
            .map(|(id, _)| id.clone())
            .collect();

        let mut newly_ready = Vec::new();
        for (id, state) in self.task_states.iter_mut() {
            if let ImplementationTaskStatus::Blocked { waiting_on } = &state.status {
                if waiting_on.iter().all(|dep| completed.contains(dep)) {
                    state.status = ImplementationTaskStatus::Pending;
                    newly_ready.push(id.clone());
                }
            }
        }
        newly_ready.sort();
        newly_ready
    }

    pub fn running_count(&self) -> usize {
        self.task_states
            .values()
            .filter(|s| matches!(s.status, ImplementationTaskStatus::Running { .. }))
            .count()
    }

    pub fn completed_count(&self) -> usize {
        self.task_states
            .values()
            .filter(|s| matches!(s.status, ImplementationTaskStatus::Complete { .. }))
            .count()
    }

    pub fn is_done(&self) -> bool {
        self.task_states.values().all(|s| {
            matches!(
                s.status,
                ImplementationTaskStatus::Complete { .. }
                    | ImplementationTaskStatus::Failed { .. }
                    | ImplementationTaskStatus::Conflict { .. }
            )
        })
    }

    pub fn total_tasks(&self) -> usize {
        self.task_states.len()
    }

    pub fn failed_tasks(&self) -> Vec<String> {
        let mut failed: Vec<String> = self
            .task_states
            .iter()
            .filter(|(_, s)| {
                matches!(
                    s.status,
                    ImplementationTaskStatus::Failed { .. }
                        | ImplementationTaskStatus::Conflict { .. }
                )
            })
            .map(|(id, _)| id.clone())
            .collect();
        failed.sort();
        failed
    }

    pub async fn spawn_task(
        &mut self,
        task_id: &str,
        get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
        tx: mpsc::Sender<TaskEvent>,
    ) -> Result<()> {
        let branch_name = self
            .task_states
            .get(task_id)
            .map(|s| s.branch_name.clone())
            .context("task not found")?;

        let dep_ids: Vec<String> = self
            .task_states
            .get(task_id)
            .map(|s| s.task.depends_on.clone())
            .unwrap_or_default();

        let dep_branches: Vec<String> = dep_ids
            .iter()
            .filter_map(|dep_id| self.task_states.get(dep_id))
            .map(|s| s.branch_name.clone())
            .collect();

        if let Some(state) = self.task_states.get_mut(task_id) {
            state.attempts += 1;
        }

        let worktree_path = self
            .worktree_manager
            .create_worktree(task_id, &branch_name)
            .await?;

        if !dep_branches.is_empty() {
            self.worktree_manager
                .merge_dependency_branches(&worktree_path, &dep_branches)
                .await?;
        }

        let provider = get_provider(&self.config.agents.implementor.provider)
            .context("no provider available for implementor")?;

        let prompt = self.build_task_prompt(task_id)?;

        let task_dir = self
            .run_dir
            .join("implementation")
            .join(format!("task-{}", task_id));
        std::fs::create_dir_all(&task_dir)?;
        std::fs::write(task_dir.join("prompt.md"), &prompt)?;

        let provider_name = provider.name().to_string();

        if let Some(state) = self.task_states.get_mut(task_id) {
            state.status = ImplementationTaskStatus::Running {
                worktree_path: worktree_path.clone(),
                provider: provider_name.clone(),
            };
        }

        tx.send(TaskEvent::Started {
            task_id: task_id.to_string(),
            provider: provider_name,
            worktree_path: worktree_path.clone(),
        })
        .await
        .ok();

        let task_id_owned = task_id.to_string();
        let no_flags: Vec<String> = vec![];
        let wt_path = worktree_path;
        let timeout = Some(Duration::from_secs(
            self.config.agents.implementor.timeout_seconds,
        ));

        let code_reviewer_provider = if self.config.agents.code_reviewer.enabled {
            get_provider(&self.config.agents.code_reviewer.provider)
        } else {
            None
        };
        let code_security_provider = if self.config.agents.code_security_auditor.enabled {
            get_provider(&self.config.agents.code_security_auditor.provider)
        } else {
            None
        };
        let code_reviewer_timeout =
            Duration::from_secs(self.config.agents.code_reviewer.timeout_seconds);
        let code_security_timeout =
            Duration::from_secs(self.config.agents.code_security_auditor.timeout_seconds);
        let run_dir_clone = self.run_dir.clone();
        let code_reviewer_custom = self.config.agents.code_reviewer.custom_instructions.clone();
        let code_security_custom = self
            .config
            .agents
            .code_security_auditor
            .custom_instructions
            .clone();
        let judge_provider = get_provider(&self.config.agents.judge.provider);
        let judge_timeout = Duration::from_secs(self.config.agents.judge.timeout_seconds);
        let judge_custom = self.config.agents.judge.custom_instructions.clone();
        let max_review_iterations = self.config.review_loop.max_iterations;

        tokio::spawn(async move {
            let start = Instant::now();
            let result = provider.run(&prompt, &wt_path, &no_flags, timeout).await;
            let duration = start.elapsed().as_secs();

            match result {
                Ok(output) => {
                    let _ = std::fs::write(task_dir.join("output.md"), &output.text);

                    let result_path = wt_path.join("TASK_RESULT.md");
                    if result_path.exists() {
                        let _ = std::fs::copy(&result_path, task_dir.join("TASK_RESULT.md"));
                    }

                    let task_result_text =
                        std::fs::read_to_string(task_dir.join("TASK_RESULT.md")).ok();

                    let parsed = task_result_text
                        .as_ref()
                        .and_then(|t| output_parser::parse_task_result(t));

                    match parsed {
                        Some(tr) => {
                            if tr.status == TaskStatus::Complete {
                                run_code_review_loop(
                                    &task_id_owned,
                                    &wt_path,
                                    &task_dir,
                                    &run_dir_clone,
                                    &no_flags,
                                    max_review_iterations,
                                    code_reviewer_provider.as_deref(),
                                    code_security_provider.as_deref(),
                                    code_reviewer_timeout,
                                    code_security_timeout,
                                    code_reviewer_custom.as_deref(),
                                    code_security_custom.as_deref(),
                                    judge_provider.as_deref(),
                                    judge_timeout,
                                    judge_custom.as_deref(),
                                    provider.as_ref(),
                                    timeout,
                                )
                                .await;
                            }

                            let _ = tx
                                .send(TaskEvent::Completed {
                                    task_id: task_id_owned,
                                    result: tr,
                                    duration_secs: duration,
                                    files_changed: 0,
                                })
                                .await;
                        }
                        None => {
                            let _ = tx
                                .send(TaskEvent::Failed {
                                    task_id: task_id_owned,
                                    error: "no parseable TASK_RESULT.md".to_string(),
                                    attempts: 1,
                                })
                                .await;
                        }
                    }
                }
                Err(e) => {
                    let _ = tx
                        .send(TaskEvent::Failed {
                            task_id: task_id_owned,
                            error: e.to_string(),
                            attempts: 1,
                        })
                        .await;
                }
            }
        });

        Ok(())
    }

    fn build_task_prompt(&self, task_id: &str) -> Result<String> {
        let state = self.task_states.get(task_id).context("task not found")?;
        let task = &state.task;

        let test_spec = self
            .test_strategy
            .per_task
            .get(task_id)
            .map(|ts| serde_json::to_string_pretty(ts).unwrap_or_default())
            .unwrap_or_default();

        context::build_implementor_prompt(task, &test_spec)
    }

    pub fn task_states(&self) -> &HashMap<String, TaskState> {
        &self.task_states
    }

    pub fn handle_event(&mut self, event: &TaskEvent) {
        match event {
            TaskEvent::Completed {
                task_id,
                duration_secs,
                files_changed,
                result,
            } => {
                if let Some(state) = self.task_states.get_mut(task_id) {
                    match result.status {
                        TaskStatus::Complete => {
                            state.status = ImplementationTaskStatus::Complete {
                                duration_secs: *duration_secs,
                                files_changed: *files_changed,
                            };
                        }
                        TaskStatus::Failed => {
                            state.status = ImplementationTaskStatus::Failed {
                                error: "task reported failure".to_string(),
                                attempts: state.attempts,
                            };
                        }
                        TaskStatus::Conflict => {
                            state.status = ImplementationTaskStatus::Conflict {
                                details: result.conflicts.join(", "),
                            };
                        }
                    }
                }
            }
            TaskEvent::Failed {
                task_id,
                error,
                attempts,
            } => {
                if let Some(state) = self.task_states.get_mut(task_id) {
                    state.status = ImplementationTaskStatus::Failed {
                        error: error.clone(),
                        attempts: *attempts,
                    };
                }
            }
            TaskEvent::Started { .. } => {}
        }
    }

    pub fn merge_order(&self) -> &[String] {
        &self.breakdown.merge_order
    }
}

async fn get_worktree_diff(worktree_path: &Path) -> String {
    let output = tokio::process::Command::new("git")
        .current_dir(worktree_path)
        .args(["diff", "HEAD~1", "--no-color"])
        .output()
        .await;

    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_code_review_loop(
    task_id: &str,
    wt_path: &Path,
    task_dir: &Path,
    run_dir: &Path,
    no_flags: &[String],
    max_iterations: u32,
    code_reviewer: Option<&dyn Provider>,
    code_security: Option<&dyn Provider>,
    reviewer_timeout: Duration,
    security_timeout: Duration,
    reviewer_custom: Option<&Path>,
    security_custom: Option<&Path>,
    judge: Option<&dyn Provider>,
    judge_timeout: Duration,
    judge_custom: Option<&Path>,
    implementor: &dyn Provider,
    implementor_timeout: Option<Duration>,
) {
    // If code_reviewer is disabled (None), skip the entire code review loop (auto-approve)
    let Some(reviewer) = code_reviewer else {
        return;
    };

    let mut previous_review_text = String::new();

    for iteration in 1..=max_iterations {
        let diff = get_worktree_diff(wt_path).await;
        if diff.is_empty() {
            return;
        }

        let review_prompt =
            context::build_code_reviewer_prompt(run_dir, task_id, &diff, reviewer_custom, wt_path);

        let review_prompt = match review_prompt {
            Ok(r) => r,
            _ => return,
        };

        let (review_text, security_text) = if let Some(security) = code_security {
            let security_prompt = context::build_code_security_auditor_prompt(
                run_dir,
                task_id,
                &diff,
                security_custom,
                wt_path,
            );

            let security_prompt = match security_prompt {
                Ok(s) => s,
                _ => return,
            };

            let (review_result, security_result) = tokio::join!(
                reviewer.run(
                    &review_prompt.prompt,
                    wt_path,
                    no_flags,
                    Some(reviewer_timeout),
                ),
                security.run(
                    &security_prompt.prompt,
                    wt_path,
                    no_flags,
                    Some(security_timeout),
                ),
            );

            (
                review_result
                    .as_ref()
                    .map(|o| o.text.clone())
                    .unwrap_or_default(),
                security_result
                    .as_ref()
                    .map(|o| o.text.clone())
                    .unwrap_or_default(),
            )
        } else {
            let review_result = reviewer
                .run(
                    &review_prompt.prompt,
                    wt_path,
                    no_flags,
                    Some(reviewer_timeout),
                )
                .await;

            (
                review_result
                    .as_ref()
                    .map(|o| o.text.clone())
                    .unwrap_or_default(),
                String::new(),
            )
        };

        let review_dir = task_dir.join(format!("code-review-{}", iteration));
        let _ = std::fs::create_dir_all(&review_dir);

        let _ = std::fs::write(review_dir.join("code-review.md"), &review_text);
        let _ = std::fs::write(review_dir.join("code-security.md"), &security_text);

        // Stall detection: break if code review is cycling
        let combined_review = format!("{}\n{}", review_text, security_text);
        if iteration > 1
            && crate::pipeline::stall::is_cycling(
                &previous_review_text,
                &combined_review,
                crate::pipeline::stall::DEFAULT_CYCLE_THRESHOLD,
            )
        {
            let _ = std::fs::write(
                review_dir.join("WARNING.md"),
                "Code review cycling detected — same findings repeating. Breaking loop.",
            );
            return;
        }
        previous_review_text = combined_review;

        let Some(judge_provider) = judge else {
            let review_summary = output_parser::parse_code_review(&review_text);
            let security_summary = output_parser::parse_code_security_review(&security_text);
            let has_any_findings = review_summary
                .as_ref()
                .is_some_and(|s| !s.findings.is_empty())
                || security_summary
                    .as_ref()
                    .is_some_and(|s| !s.findings.is_empty());
            if !has_any_findings {
                return;
            }
            if iteration >= max_iterations {
                let _ = std::fs::write(
                    review_dir.join("WARNING.md"),
                    "Code review found issues but max iterations reached (no judge configured).",
                );
                return;
            }
            let fix_context = format!(
                "## Code Review Findings (Iteration {})\n\n\
                 You must fix the findings below before your task is complete.\n\n\
                 ### Code Review\n\n{}\n\n\
                 ### Code Security Review\n\n{}",
                iteration, review_text, security_text
            );
            let _ = implementor
                .run(&fix_context, wt_path, no_flags, implementor_timeout)
                .await;
            continue;
        };

        let judge_prompt = context::build_code_judge_prompt(
            run_dir,
            task_id,
            iteration,
            &diff,
            &review_text,
            &security_text,
            judge_custom,
            wt_path,
        );

        let judge_prompt = match judge_prompt {
            Ok(p) => p,
            _ => return,
        };

        let judge_result = judge_provider
            .run(&judge_prompt.prompt, wt_path, no_flags, Some(judge_timeout))
            .await;

        let judge_text = judge_result
            .as_ref()
            .map(|o| o.text.clone())
            .unwrap_or_default();

        let _ = std::fs::write(review_dir.join("judgment.md"), &judge_text);

        let verdict = output_parser::parse_verdict(&judge_text);

        match verdict {
            Some(v) => {
                if v.overall == "APPROVE" || v.valid_count == 0 {
                    return;
                }

                if iteration >= max_iterations {
                    let _ = std::fs::write(
                        review_dir.join("WARNING.md"),
                        format!(
                            "Code review judge found {} valid issue(s) but max iterations reached.",
                            v.valid_count
                        ),
                    );
                    return;
                }

                let fix_context = format!(
                    "## Code Review Findings (Iteration {})\n\n\
                     The judge evaluated the code review findings and determined {} issue(s) are valid.\n\
                     You must fix the VALID findings below before your task is complete.\n\n\
                     ### Judge Verdict\n\n{}\n\n\
                     ### Code Review\n\n{}\n\n\
                     ### Code Security Review\n\n{}",
                    iteration, v.valid_count, judge_text, review_text, security_text
                );

                let _ = implementor
                    .run(&fix_context, wt_path, no_flags, implementor_timeout)
                    .await;
            }
            None => {
                if iteration >= max_iterations {
                    let _ = std::fs::write(
                        review_dir.join("WARNING.md"),
                        "Judge did not produce a parseable verdict and max iterations reached.",
                    );
                }
                return;
            }
        }
    }
}
