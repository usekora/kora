# Kora Phase 3: Planning & Implementation Pipeline

**Goal:** Build the planner agent, test architect agent, implementor fleet with parallel execution in git worktrees, and the implementation dashboard UI. Wire everything into the orchestrator so the pipeline flows from approved plan through planning, test architecting, and parallel implementation.

**Spec:** `docs/specs/2026-03-22-kora-design.md`

**Depends on:** Phase 2 (complete) -- core pipeline with researcher, reviewer, security auditor, judge, review loop.

---

## File Structure (new and modified files)

```
kora/
├── src/
│   ├── lib.rs                           ← MODIFY (add git module)
│   ├── git/
│   │   ├── mod.rs                       ← NEW (git module exports)
│   │   └── worktree.rs                  ← NEW (git worktree create/cleanup/merge via tokio::process::Command)
│   ├── pipeline/
│   │   ├── mod.rs                       ← MODIFY (add planner, test_architect, implementation modules)
│   │   ├── orchestrator.rs              ← MODIFY (wire planner -> test architect -> implementors after review loop)
│   │   ├── planner.rs                   ← NEW (run planner agent, parse task-breakdown.json)
│   │   ├── test_architect.rs            ← NEW (run test architect agent, parse test-strategy.json)
│   │   ├── implementation.rs            ← NEW (implementor fleet: spawn parallel agents, dependency graph, retry)
│   │   └── context.rs                   ← MODIFY (add planner, test architect, implementor prompt builders)
│   ├── terminal/
│   │   ├── mod.rs                       ← MODIFY (add dashboard module)
│   │   ├── renderer.rs                  ← MODIFY (add implementation-phase rendering methods)
│   │   └── dashboard.rs                 ← NEW (live implementation dashboard with crossterm cursor manipulation)
│   ├── agent/
│   │   └── output_parser.rs             ← MODIFY (add parse_task_breakdown, parse_test_strategy, parse_task_result)
├── tests/
│   ├── worktree_test.rs                 ← NEW (git worktree management tests)
│   ├── planner_test.rs                  ← NEW (planner output parsing + context tests)
│   ├── test_architect_test.rs           ← NEW (test architect output parsing + context tests)
│   ├── implementation_test.rs           ← NEW (implementor fleet dependency resolution + scheduling tests)
│   ├── dashboard_test.rs                ← NEW (dashboard state management tests)
│   ├── context_test.rs                  ← MODIFY (add planner/test-architect/implementor prompt tests)
│   └── output_parser_test.rs            ← MODIFY (add task breakdown + test strategy + task result parsing tests)
```

---

### Task 1: Data Types for Planner and Test Architect Output

Add serde-deserializable types for `task-breakdown.json` and `test-strategy.json` to the output parser, plus a `TaskResult` type parsed from `TASK_RESULT.md`.

**Files:**
- Modify: `src/agent/output_parser.rs`
- Modify: `tests/output_parser_test.rs`

**Implementation:**

```rust
// In output_parser.rs -- add:

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBreakdown {
    pub tasks: Vec<Task>,
    pub branch_strategy: String,
    pub merge_order: Vec<String>,
    pub critical_path: Vec<String>,
    pub parallelism_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub files: TaskFiles,
    pub depends_on: Vec<String>,
    pub estimated_complexity: String,
    #[serde(default)]
    pub conflict_risk: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFiles {
    pub create: Vec<String>,
    pub modify: Vec<String>,
    pub delete: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestStrategy {
    pub per_task: std::collections::HashMap<String, TaskTestSpec>,
    pub post_merge: PostMergeTests,
    pub testing_patterns: TestingPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTestSpec {
    pub unit_tests: Vec<TestSpec>,
    pub integration_tests: Vec<TestSpec>,
    pub edge_case_tests: Vec<TestSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSpec {
    pub description: String,
    pub file: String,
    pub setup: String,
    pub expected: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMergeTests {
    pub integration_tests: Vec<PostMergeTestSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostMergeTestSpec {
    pub description: String,
    pub tasks_involved: Vec<String>,
    pub setup: String,
    pub expected: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingPatterns {
    pub framework: String,
    pub conventions: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskStatus {
    Complete,
    Failed,
    Conflict,
}

#[derive(Debug, Clone)]
pub struct TaskResult {
    pub status: TaskStatus,
    pub changes: Vec<String>,
    pub tests_written: u32,
    pub tests_passing: u32,
    pub tests_failing: u32,
    pub conflicts: Vec<String>,
    pub observations: Vec<String>,
}

pub fn parse_task_breakdown(text: &str) -> Result<TaskBreakdown, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn parse_test_strategy(text: &str) -> Result<TestStrategy, serde_json::Error> {
    serde_json::from_str(text)
}

pub fn parse_task_result(text: &str) -> Option<TaskResult> {
    // Parse TASK_RESULT.md format: extract Status line, Changes, Tests, Conflicts, Observations
}
```

**Tests:**
- Parse valid task-breakdown.json
- Parse valid test-strategy.json
- Parse TASK_RESULT.md with COMPLETE status
- Parse TASK_RESULT.md with FAILED status
- Parse TASK_RESULT.md with CONFLICT status
- Reject malformed JSON for task breakdown
- Reject malformed JSON for test strategy
- Handle TASK_RESULT.md with missing sections gracefully

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all existing + new tests pass

---

### Task 2: Git Worktree Management Module

Create `src/git/` module with worktree lifecycle management using `tokio::process::Command` to shell out to git.

**Files:**
- Modify: `src/lib.rs` (add `pub mod git;`)
- Create: `src/git/mod.rs`
- Create: `src/git/worktree.rs`
- Create: `tests/worktree_test.rs`

**Implementation:**

```rust
// src/git/worktree.rs

use std::path::{Path, PathBuf};
use anyhow::Result;
use tokio::process::Command;

pub struct WorktreeManager {
    repo_root: PathBuf,
}

impl WorktreeManager {
    pub fn new(repo_root: &Path) -> Self { ... }

    /// Create a new worktree for a task, branching from the current branch.
    /// Returns the path to the new worktree.
    pub async fn create_worktree(
        &self,
        task_id: &str,
        branch_name: &str,
    ) -> Result<PathBuf> {
        // git worktree add ../kora-worktree-{task_id} -b {branch_name}
        // Return the worktree path
    }

    /// Merge dependency branches into a worktree.
    /// Called when a blocked task unblocks and its dependency branches need to be merged in.
    pub async fn merge_dependency_branches(
        &self,
        worktree_path: &Path,
        dependency_branches: &[String],
    ) -> Result<()> {
        // For each dependency branch:
        //   git -C {worktree_path} merge {branch} --no-edit
        // If merge fails, return error with conflict details
    }

    /// Remove a worktree and its branch after task completion.
    pub async fn remove_worktree(&self, worktree_path: &Path) -> Result<()> {
        // git worktree remove {worktree_path} --force
    }

    /// List all kora-managed worktrees.
    pub async fn list_worktrees(&self) -> Result<Vec<WorktreeInfo>> {
        // git worktree list --porcelain
        // Filter to kora-worktree-* entries
    }

    /// Clean up all kora-managed worktrees.
    pub async fn cleanup_all(&self) -> Result<()> {
        // List and remove all kora-worktree-* entries
    }

    /// Get the current branch name.
    pub async fn current_branch(&self) -> Result<String> {
        // git rev-parse --abbrev-ref HEAD
    }

    /// Create a branch for a task (without worktree, for single-branch strategy).
    pub async fn create_branch(&self, branch_name: &str) -> Result<()> {
        // git branch {branch_name}
    }

    /// Merge a task branch into the target branch.
    pub async fn merge_branch(
        &self,
        target_dir: &Path,
        branch_name: &str,
    ) -> Result<MergeResult> {
        // git -C {target_dir} merge {branch_name} --no-edit
        // Return success/conflict info
    }
}

pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
    pub task_id: Option<String>,
}

pub enum MergeResult {
    Success,
    Conflict { files: Vec<String> },
}
```

**Tests:**
- Create and remove worktrees in a temp git repo
- List worktrees returns newly created ones
- Cleanup removes all kora-managed worktrees
- Current branch detection works
- Merge branch returns success for clean merges
- Merge branch returns conflict for conflicting changes

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all tests pass (uses real git in temp dirs)

---

### Task 3: Planner Agent Pipeline Stage

Create `src/pipeline/planner.rs` -- runs the planner agent and parses its output into a `TaskBreakdown`.

**Files:**
- Create: `src/pipeline/planner.rs`
- Modify: `src/pipeline/mod.rs` (add `pub mod planner;`)
- Modify: `src/pipeline/context.rs` (add `build_planner_prompt`)
- Create: `tests/planner_test.rs`
- Modify: `tests/context_test.rs` (add planner prompt test)

**Implementation:**

```rust
// src/pipeline/planner.rs

use anyhow::{Context, Result};
use std::path::Path;

use crate::agent::output_parser::{self, TaskBreakdown};
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

    // Save raw output
    let plan_dir = run_dir.plan_dir();
    std::fs::create_dir_all(&plan_dir)?;
    std::fs::write(plan_dir.join("planner-output.md"), &output.text)?;

    // Try to parse JSON from the output
    // Agent may include explanation text around the JSON; extract the JSON object
    let json_text = extract_json_object(&output.text)
        .context("planner output does not contain valid JSON")?;

    let breakdown: TaskBreakdown = output_parser::parse_task_breakdown(&json_text)
        .context("failed to parse task breakdown JSON")?;

    // Validate: tasks must have unique IDs, dependencies must reference existing tasks
    validate_breakdown(&breakdown)?;

    std::fs::write(
        plan_dir.join("task-breakdown.json"),
        serde_json::to_string_pretty(&breakdown)?,
    )?;

    Ok(breakdown)
}

fn extract_json_object(text: &str) -> Option<String> {
    // Find first '{' and last '}' to extract the JSON object
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    if end > start {
        Some(text[start..=end].to_string())
    } else {
        None
    }
}

fn validate_breakdown(breakdown: &TaskBreakdown) -> Result<()> {
    let ids: std::collections::HashSet<&str> = breakdown.tasks.iter().map(|t| t.id.as_str()).collect();

    // Check unique IDs
    if ids.len() != breakdown.tasks.len() {
        anyhow::bail!("task breakdown contains duplicate task IDs");
    }

    // Check dependencies reference existing tasks
    for task in &breakdown.tasks {
        for dep in &task.depends_on {
            if !ids.contains(dep.as_str()) {
                anyhow::bail!("task {} depends on non-existent task {}", task.id, dep);
            }
        }
    }

    // Check no self-dependencies
    for task in &breakdown.tasks {
        if task.depends_on.contains(&task.id) {
            anyhow::bail!("task {} depends on itself", task.id);
        }
    }

    // Check merge_order references existing tasks
    for id in &breakdown.merge_order {
        if !ids.contains(id.as_str()) {
            anyhow::bail!("merge_order references non-existent task {}", id);
        }
    }

    Ok(())
}
```

Context builder in `context.rs`:

```rust
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
    let codebase_summary = read_file_if_exists(&run_dir.join("context").join("codebase-summary.md"))
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
```

**Tests:**
- `extract_json_object` finds JSON in mixed text
- `validate_breakdown` rejects duplicate IDs
- `validate_breakdown` rejects missing dependency references
- `validate_breakdown` rejects self-dependencies
- `build_planner_prompt` includes plan and codebase summary

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all tests pass

---

### Task 4: Test Architect Agent Pipeline Stage

Create `src/pipeline/test_architect.rs` -- runs the test architect agent and parses its output into a `TestStrategy`.

**Files:**
- Create: `src/pipeline/test_architect.rs`
- Modify: `src/pipeline/mod.rs` (add `pub mod test_architect;`)
- Modify: `src/pipeline/context.rs` (add `build_test_architect_prompt`)
- Create: `tests/test_architect_test.rs`
- Modify: `tests/context_test.rs` (add test architect prompt test)

**Implementation:**

```rust
// src/pipeline/test_architect.rs

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
```

Context builder:

```rust
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
    let codebase_summary = read_file_if_exists(&run_dir.join("context").join("codebase-summary.md"))
        .unwrap_or_default();
    let task_breakdown = read_file_if_exists(&run_dir.join("plan").join("task-breakdown.json"))
        .unwrap_or_default();

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
```

**Tests:**
- `build_test_architect_prompt` includes task breakdown and plan
- Valid test strategy JSON parses correctly

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all tests pass

---

### Task 5: Implementor Fleet with Parallel Execution

Create `src/pipeline/implementation.rs` -- the core of Phase 3. Manages the dependency graph, spawns parallel implementor agents capped at `parallel_limit`, handles task lifecycle.

**Files:**
- Create: `src/pipeline/implementation.rs`
- Modify: `src/pipeline/mod.rs` (add `pub mod implementation;`)
- Modify: `src/pipeline/context.rs` (add `build_implementor_prompt`)
- Create: `tests/implementation_test.rs`

**Implementation:**

```rust
// src/pipeline/implementation.rs

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use anyhow::{Context as AnyhowContext, Result};

use crate::agent::output_parser::{self, Task, TaskBreakdown, TaskResult, TaskStatus, TestStrategy};
use crate::config::Config;
use crate::git::worktree::WorktreeManager;
use crate::pipeline::context;
use crate::provider::Provider;
use crate::state::RunDirectory;

#[derive(Debug, Clone, PartialEq)]
pub enum ImplementationTaskStatus {
    Pending,
    Blocked { waiting_on: Vec<String> },
    Running { worktree_path: PathBuf, provider: String },
    Complete { duration_secs: u64, files_changed: u32 },
    Failed { error: String, attempts: u32 },
    Conflict { details: String },
}

#[derive(Debug, Clone)]
pub struct TaskState {
    pub task: Task,
    pub status: ImplementationTaskStatus,
    pub branch_name: String,
    pub attempts: u32,
}

pub enum TaskEvent {
    Started { task_id: String, provider: String, worktree_path: PathBuf },
    Completed { task_id: String, result: TaskResult, duration_secs: u64, files_changed: u32 },
    Failed { task_id: String, error: String, attempts: u32 },
    ProgressTick { task_id: String },
}

pub struct ImplementationFleet {
    config: Config,
    breakdown: TaskBreakdown,
    test_strategy: TestStrategy,
    worktree_manager: WorktreeManager,
    run_dir: RunDirectory,
    project_root: PathBuf,
    task_states: HashMap<String, TaskState>,
}

impl ImplementationFleet {
    pub fn new(
        config: Config,
        breakdown: TaskBreakdown,
        test_strategy: TestStrategy,
        project_root: &Path,
        run_dir: RunDirectory,
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
            task_states.insert(task.id.clone(), TaskState {
                task: task.clone(),
                status,
                branch_name,
                attempts: 0,
            });
        }

        Self {
            config,
            breakdown,
            test_strategy,
            worktree_manager,
            run_dir,
            project_root: project_root.to_path_buf(),
            task_states,
        }
    }

    /// Get tasks that are ready to run (pending, not blocked).
    pub fn ready_tasks(&self) -> Vec<String> {
        self.task_states
            .iter()
            .filter(|(_, s)| matches!(s.status, ImplementationTaskStatus::Pending))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Get tasks that are currently blocked and check if they can be unblocked.
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
        newly_ready
    }

    /// Count of running tasks.
    pub fn running_count(&self) -> usize {
        self.task_states
            .values()
            .filter(|s| matches!(s.status, ImplementationTaskStatus::Running { .. }))
            .count()
    }

    /// Whether all tasks are complete or failed.
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

    /// Run the full implementation fleet. Returns the event receiver for dashboard updates.
    pub async fn run(
        &mut self,
        get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    ) -> Result<mpsc::Receiver<TaskEvent>> {
        let (tx, rx) = mpsc::channel(64);
        let parallel_limit = self.config.implementation.parallel_limit as usize;

        loop {
            // Check for newly unblocked tasks
            self.check_unblocked();

            // Spawn tasks up to parallel limit
            let available_slots = parallel_limit.saturating_sub(self.running_count());
            let mut ready = self.ready_tasks();
            ready.truncate(available_slots);

            for task_id in ready {
                self.spawn_task(&task_id, get_provider, tx.clone()).await?;
            }

            if self.is_done() {
                break;
            }

            // Wait for any task to complete (via event channel)
            // This is driven by the tokio tasks completing and sending events
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        Ok(rx)
    }

    async fn spawn_task(
        &mut self,
        task_id: &str,
        get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
        tx: mpsc::Sender<TaskEvent>,
    ) -> Result<()> {
        let state = self.task_states.get_mut(task_id).unwrap();
        state.attempts += 1;

        // Create worktree
        let worktree_path = self
            .worktree_manager
            .create_worktree(task_id, &state.branch_name)
            .await?;

        // Merge dependency branches if any
        let dep_branches: Vec<String> = state
            .task
            .depends_on
            .iter()
            .filter_map(|dep_id| self.task_states.get(dep_id))
            .map(|s| s.branch_name.clone())
            .collect();

        if !dep_branches.is_empty() {
            self.worktree_manager
                .merge_dependency_branches(&worktree_path, &dep_branches)
                .await?;
        }

        let provider = get_provider(&self.config.agents.implementor.provider)
            .context("no provider available for implementor")?;

        // Build prompt
        let prompt = self.build_task_prompt(task_id)?;

        // Save prompt to run dir
        let task_dir = self.run_dir.task_dir(task_id);
        std::fs::create_dir_all(&task_dir)?;
        std::fs::write(task_dir.join("prompt.md"), &prompt)?;

        let provider_name = provider.name().to_string();
        state.status = ImplementationTaskStatus::Running {
            worktree_path: worktree_path.clone(),
            provider: provider_name.clone(),
        };

        tx.send(TaskEvent::Started {
            task_id: task_id.to_string(),
            provider: provider_name,
            worktree_path: worktree_path.clone(),
        })
        .await
        .ok();

        // Spawn the agent as a tokio task
        let task_id_owned = task_id.to_string();
        let no_flags: Vec<String> = vec![];
        let tx_clone = tx.clone();
        let task_dir_clone = task_dir.clone();
        let wt_path = worktree_path.clone();

        tokio::spawn(async move {
            let start = std::time::Instant::now();
            let result = provider.run(&prompt, &wt_path, &no_flags).await;
            let duration = start.elapsed().as_secs();

            match result {
                Ok(output) => {
                    // Save output
                    let _ = std::fs::write(task_dir_clone.join("output.md"), &output.text);

                    // Copy TASK_RESULT.md from worktree if it exists
                    let result_path = wt_path.join("TASK_RESULT.md");
                    if result_path.exists() {
                        let _ = std::fs::copy(&result_path, task_dir_clone.join("TASK_RESULT.md"));
                    }

                    // Parse task result
                    let task_result_text = std::fs::read_to_string(
                        task_dir_clone.join("TASK_RESULT.md"),
                    )
                    .ok();

                    let parsed = task_result_text
                        .as_ref()
                        .and_then(|t| output_parser::parse_task_result(t));

                    match parsed {
                        Some(tr) => {
                            let _ = tx_clone
                                .send(TaskEvent::Completed {
                                    task_id: task_id_owned,
                                    result: tr,
                                    duration_secs: duration,
                                    files_changed: 0,
                                })
                                .await;
                        }
                        None => {
                            let _ = tx_clone
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
                    let _ = tx_clone
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
        let state = self.task_states.get(task_id).unwrap();
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

    /// Handle task event -- update internal state.
    pub fn handle_event(&mut self, event: &TaskEvent) {
        match event {
            TaskEvent::Completed { task_id, duration_secs, files_changed, result } => {
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
            TaskEvent::Failed { task_id, error, attempts } => {
                if let Some(state) = self.task_states.get_mut(task_id) {
                    state.status = ImplementationTaskStatus::Failed {
                        error: error.clone(),
                        attempts: *attempts,
                    };
                }
            }
            _ => {}
        }
    }
}
```

Context builder for implementor:

```rust
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
        task.files.create.join(", "),
        task.files.modify.join(", "),
        task.files.delete.join(", "),
    );

    let test_section = format!("## Test Requirements\n\n{}", test_spec);

    let full_prompt = format!("{}\n\n---\n\n{}\n\n---\n\n{}", base, task_section, test_section);
    Ok(full_prompt)
}
```

**Tests (implementation_test.rs):**
- `ready_tasks` returns only tasks with no dependencies
- `check_unblocked` transitions blocked tasks to pending when deps complete
- `is_done` returns false when tasks are still pending/running
- `is_done` returns true when all tasks are complete/failed
- `handle_event` updates task state correctly for completed events
- `handle_event` updates task state correctly for failed events
- Dependency graph with chain T1->T2->T3: only T1 starts initially
- Multiple independent tasks all appear as ready
- `running_count` reflects spawned tasks

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all tests pass

---

### Task 6: Implementation Dashboard

Create `src/terminal/dashboard.rs` -- a live, in-place updating dashboard for the implementation phase using crossterm cursor manipulation.

**Files:**
- Create: `src/terminal/dashboard.rs`
- Modify: `src/terminal/mod.rs` (add `pub mod dashboard;`)
- Modify: `src/terminal/renderer.rs` (add implementation-phase methods)
- Create: `tests/dashboard_test.rs`

**Implementation:**

```rust
// src/terminal/dashboard.rs

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use crossterm::{cursor, execute, style::*, terminal};

use crate::pipeline::implementation::{ImplementationTaskStatus, TaskState};

pub struct Dashboard {
    stdout: io::Stdout,
    task_order: Vec<String>,
    last_render_lines: u16,
    total_tasks: usize,
    show_task: Option<String>,
}

impl Dashboard {
    pub fn new(task_order: Vec<String>) -> Self {
        let total = task_order.len();
        Self {
            stdout: io::stdout(),
            task_order,
            last_render_lines: 0,
            total_tasks: total,
            show_task: None,
        }
    }

    /// Render the dashboard. Clears previous render and redraws in-place.
    pub fn render(&mut self, task_states: &HashMap<String, TaskState>) {
        // Move cursor up to overwrite previous render
        if self.last_render_lines > 0 {
            execute!(
                self.stdout,
                cursor::MoveUp(self.last_render_lines),
                terminal::Clear(terminal::ClearType::FromCursorDown),
            )
            .ok();
        }

        let completed = task_states
            .values()
            .filter(|s| matches!(s.status, ImplementationTaskStatus::Complete { .. }))
            .count();

        // Header line
        let header = format!(
            "  implementing {} of {} ",
            completed, self.total_tasks
        );
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(&header),
            SetAttribute(Attribute::Reset),
            ResetColor,
            Print("\n\n"),
        )
        .ok();

        let mut lines: u16 = 3;

        for task_id in &self.task_order {
            if let Some(state) = task_states.get(task_id) {
                self.render_task_line(state);
                lines += 1;
            }
        }

        execute!(self.stdout, Print("\n")).ok();
        lines += 1;

        self.last_render_lines = lines;
        self.stdout.flush().ok();
    }

    fn render_task_line(&mut self, state: &TaskState) {
        let id = &state.task.id;
        let branch = &state.branch_name;

        let (status_text, color, progress_bar) = match &state.status {
            ImplementationTaskStatus::Pending => {
                ("pending".to_string(), Color::DarkGrey, render_bar(0))
            }
            ImplementationTaskStatus::Blocked { waiting_on } => {
                let deps = waiting_on.join(",");
                (format!("blocked -> {}", deps), Color::DarkGrey, render_bar(0))
            }
            ImplementationTaskStatus::Running { provider, .. } => {
                (format!("running  {}", provider), Color::Cyan, render_bar(50))
            }
            ImplementationTaskStatus::Complete { duration_secs, files_changed } => {
                (
                    format!("done {}s  {} files", duration_secs, files_changed),
                    Color::Green,
                    render_bar(100),
                )
            }
            ImplementationTaskStatus::Failed { error, attempts } => {
                (
                    format!("FAILED (attempt {})", attempts),
                    Color::Red,
                    render_bar(0),
                )
            }
            ImplementationTaskStatus::Conflict { .. } => {
                ("CONFLICT".to_string(), Color::Yellow, render_bar(0))
            }
        };

        execute!(
            self.stdout,
            Print("    "),
            SetForegroundColor(color),
            Print(format!("{:<4}", id)),
            ResetColor,
            Print(format!(" {} ", progress_bar)),
            SetForegroundColor(color),
            Print(format!("{:<30}", status_text)),
            ResetColor,
            SetForegroundColor(Color::DarkGrey),
            Print(format!(" {}", branch)),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }

    pub fn set_show_task(&mut self, task_id: Option<String>) {
        self.show_task = task_id;
    }

    pub fn showing_task(&self) -> Option<&str> {
        self.show_task.as_deref()
    }
}

fn render_bar(percent: u8) -> String {
    let filled = (percent as usize * 12) / 100;
    let empty = 12 - filled;
    let bar: String = std::iter::repeat('█')
        .take(filled)
        .chain(std::iter::repeat('░').take(empty))
        .collect();
    bar
}
```

Additional renderer methods:

```rust
// In renderer.rs -- add:

pub fn implementation_header(&mut self, completed: usize, total: usize) {
    let dots = ".".repeat(30);
    execute!(
        self.stdout,
        Print("\n  "),
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print("implementing"),
        SetAttribute(Attribute::Reset),
        Print(" "),
        SetForegroundColor(Color::DarkGrey),
        Print(dots),
        Print(" "),
        ResetColor,
        Print(format!("{} of {} ", completed, total)),
        SetForegroundColor(Color::Green),
        Print("●"),
        ResetColor,
        Print("\n"),
    )
    .ok();
}

pub fn implementation_complete(&mut self, total_tasks: usize, total_duration_secs: u64) {
    execute!(
        self.stdout,
        Print("\n  "),
        SetForegroundColor(Color::Green),
        Print(format!(
            "all {} tasks complete in {}s",
            total_tasks, total_duration_secs
        )),
        ResetColor,
        Print("\n"),
    )
    .ok();
}

pub fn task_failure(&mut self, task_id: &str, error: &str) {
    execute!(
        self.stdout,
        Print("\n  "),
        SetForegroundColor(Color::Red),
        Print(format!("task {} failed: {}", task_id, error)),
        ResetColor,
        Print("\n"),
    )
    .ok();
}
```

**Tests (dashboard_test.rs):**
- `render_bar(0)` produces all empty blocks
- `render_bar(100)` produces all filled blocks
- `render_bar(50)` produces half and half
- `Dashboard::new` initializes with correct task count
- `set_show_task` and `showing_task` round-trip

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all tests pass

---

### Task 7: Wire Planner + Test Architect + Implementors into Orchestrator

Modify `src/pipeline/orchestrator.rs` to continue the pipeline after review loop approval: planner -> test architect -> checkpoint -> implementation fleet -> dashboard.

**Files:**
- Modify: `src/pipeline/orchestrator.rs`

**Implementation:**

Replace the placeholder `"planning + implementation pipeline coming in Phase 3"` with actual calls:

```rust
// After ReviewOutcome::Approved and checkpoint check:

// 1. Run planner
renderer.stage_header("planner", "decomposing");
run_state.advance(Stage::Planning);
run_state.save(runs_dir)?;

let planner_provider = get_provider(&config.agents.planner.provider)
    .context("no provider for planner")?;
let planner_prompt = context::build_planner_prompt(
    &runs_dir.join(&run_state.id),
    &run_state.request,
    project_root,
    config.agents.planner.custom_instructions.as_deref(),
)?;

let breakdown = planner::run_planner(
    planner_provider.as_ref(),
    &planner_prompt.prompt,
    project_root,
    &run_dir,
    &no_flags,
)
.await?;

renderer.stage_complete("planner", 0);
renderer.info(&format!(
    "{} tasks, strategy: {}, critical path: {}",
    breakdown.tasks.len(),
    breakdown.branch_strategy,
    breakdown.critical_path.join(" -> "),
));

// 2. Run test architect
renderer.stage_header("test architect", "designing tests");
run_state.advance(Stage::TestArchitecting);
run_state.save(runs_dir)?;

let ta_provider = get_provider(&config.agents.test_architect.provider)
    .context("no provider for test architect")?;
let ta_prompt = context::build_test_architect_prompt(
    &runs_dir.join(&run_state.id),
    &run_state.request,
    project_root,
    config.agents.test_architect.custom_instructions.as_deref(),
)?;

let test_strategy = test_architect::run_test_architect(
    ta_provider.as_ref(),
    &ta_prompt.prompt,
    project_root,
    &run_dir,
    &no_flags,
)
.await?;

renderer.stage_complete("test architect", 0);

// 3. Checkpoint before implementation
if should_checkpoint(&Stage::Implementing, &checkpoints)
    && !renderer.checkpoint_prompt("implementation")
{
    run_state.set_error("user declined at planner checkpoint");
    run_state.save(runs_dir)?;
    renderer.info("run cancelled by user at planner checkpoint");
    return Ok(());
}

// 4. Run implementation fleet
run_state.advance(Stage::Implementing);
run_state.save(runs_dir)?;

let mut fleet = implementation::ImplementationFleet::new(
    config.clone(),
    breakdown.clone(),
    test_strategy,
    project_root,
    RunDirectory::new(runs_dir, &run_state.id),
);

let task_order: Vec<String> = breakdown.merge_order.clone();
let mut dashboard = dashboard::Dashboard::new(task_order);

let mut rx = fleet.run(&get_provider).await?;

// Event loop: receive events, update fleet state, re-render dashboard
while let Some(event) = rx.recv().await {
    fleet.handle_event(&event);
    dashboard.render(fleet.task_states());

    if fleet.is_done() {
        break;
    }
}

// Check results
let failed_tasks: Vec<String> = fleet
    .task_states()
    .iter()
    .filter(|(_, s)| matches!(s.status, ImplementationTaskStatus::Failed { .. } | ImplementationTaskStatus::Conflict { .. }))
    .map(|(id, _)| id.clone())
    .collect();

if failed_tasks.is_empty() {
    renderer.implementation_complete(breakdown.tasks.len(), 0);
    run_state.advance(Stage::Complete);
} else {
    for task_id in &failed_tasks {
        if let Some(state) = fleet.task_states().get(task_id) {
            if let ImplementationTaskStatus::Failed { ref error, .. } = state.status {
                renderer.task_failure(task_id, error);
            }
        }
    }
    run_state.set_error(&format!("tasks failed: {}", failed_tasks.join(", ")));
}
```

**Verification:**
- `cargo build` succeeds
- Existing tests still pass

---

### Task 8: Implementor Failure Handling and Retry

Add retry logic to the implementation fleet: on first failure, retry once. On second failure with same provider, try a different provider. On third failure, escalate to user.

**Files:**
- Modify: `src/pipeline/implementation.rs` (add retry logic in the event loop)

**Implementation:**

Add to `ImplementationFleet`:

```rust
pub async fn handle_failure_with_retry(
    &mut self,
    task_id: &str,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    tx: mpsc::Sender<TaskEvent>,
) -> Result<bool> {
    let state = self.task_states.get(task_id).unwrap();
    let attempts = state.attempts;

    if attempts >= 3 {
        // Escalate -- no more retries
        return Ok(false);
    }

    // Clean up old worktree
    if let ImplementationTaskStatus::Running { ref worktree_path, .. } = state.status {
        let _ = self.worktree_manager.remove_worktree(worktree_path).await;
    }

    // Reset to pending for re-spawn
    let state = self.task_states.get_mut(task_id).unwrap();
    state.status = ImplementationTaskStatus::Pending;

    // Re-spawn (attempts counter was already incremented)
    self.spawn_task(task_id, get_provider, tx).await?;
    Ok(true)
}
```

**Tests:**
- Task retries once after first failure
- Task escalates after max attempts

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all tests pass

---

### Task 9: Integration and Final Wiring

Verify the full pipeline compiles and all modules connect. Add any missing `use` statements, fix lifetime issues, and ensure the event loop in the orchestrator properly drives the dashboard.

**Files:**
- Potentially modify any file with compilation fixes

**Verification:**
- `cargo build` succeeds
- `cargo test` -- all tests pass (50 existing + new tests)
- `cargo clippy -- -D warnings` -- no warnings

---

### Task 10: Comprehensive Tests

Ensure test coverage for all new modules. Target additions:

**Files:**
- `tests/worktree_test.rs` -- git worktree lifecycle (6 tests)
- `tests/planner_test.rs` -- planner parsing and validation (5 tests)
- `tests/test_architect_test.rs` -- test architect parsing (3 tests)
- `tests/implementation_test.rs` -- fleet scheduling and dependency graph (9 tests)
- `tests/dashboard_test.rs` -- dashboard state (5 tests)
- `tests/output_parser_test.rs` -- add task breakdown/strategy/result parsing (8 tests)
- `tests/context_test.rs` -- add planner/test-architect/implementor prompt tests (4 tests)

Total new tests: ~40

**Verification:**
- `cargo test` -- all ~90 tests pass
- `cargo clippy -- -D warnings` -- clean
