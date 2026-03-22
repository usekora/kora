# Phase 4: Validation & Polish

**Date:** 2026-03-22
**Prerequisite:** Phase 3 complete (97 tests passing)
**Scope:** Validator agent, validation loop, merge flow, conflict resolution, session management commands, inline meta commands, interactive session loop

## Overview

Phase 4 wires the final pipeline stages: after implementation completes, the validator agent checks the result against the plan, a validation loop fixes blocking issues, and a conversational merge flow lets the user decide what to do with the changes. It also implements the three session management commands (`resume`, `history`, `clean`), inline meta commands (`/status`, `/config`, `/verbose`, `/help`), and turns the default `kora` entry point into a persistent interactive session that accepts multiple runs.

## Task Breakdown

### Task 1: Validator Agent (`src/pipeline/validation.rs`)

Create the validation module that runs after implementation.

**What it does:**
- Takes the run directory, project root, and a provider
- Builds a validator prompt using `context::build_validator_prompt()` (new)
- Injects: the approved plan, task breakdown, test strategy, every task's TASK_RESULT.md, and the codebase summary
- Runs the provider non-interactively
- Saves output to `validation/report.md`
- Parses the `<!-- VALIDATION -->` block using the existing `output_parser::parse_validation()`
- Returns a `ValidationResult`

**New function in `context.rs`:**
```rust
pub fn build_validator_prompt(
    run_dir: &Path,
    request: &str,
    project_root: &Path,
    custom_instructions_path: Option<&Path>,
) -> Result<PromptContext>
```

This reads: `context/researcher-plan.md`, `context/codebase-summary.md`, `plan/task-breakdown.json`, `plan/test-strategy.json`, and all `implementation/task-*/TASK_RESULT.md` files.

**New file:** `src/pipeline/validation.rs`

```rust
pub struct ValidatorResult {
    pub validation: ValidationResult,  // from output_parser
    pub report_path: PathBuf,
}

pub async fn run_validator(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &RunDirectory,
    extra_flags: &[String],
) -> Result<ValidatorResult>
```

### Task 2: Validation Loop

After implementation, if the validator finds blocking issues, respawn targeted implementor fixes and re-validate. Max iterations from `config.validation_loop.max_iterations`.

**Changes to `orchestrator.rs`:**
- After the implementation fleet completes (and no tasks failed), call `run_validation_loop()`
- The loop:
  1. Run validator
  2. If PASS: proceed to merge flow
  3. If FAIL with blocking issues: extract the "Required Fixes" section, spawn a single implementor agent with a fix prompt in the project root (not worktrees -- the branches are already merged at this point)
  4. Re-validate
  5. If max iterations exceeded: escalate to user
- State transitions: `Implementing -> Validating`, `Validating -> Fixing -> Validating`, or `Validating -> Complete` (via merge flow)

**New function:** `run_validation_loop()` in `orchestrator.rs`

### Task 3: Merge Flow with Arrow-Key Selector

After validation passes, present the user with a conversational merge flow.

**Options (arrow-key selector):**
1. "Merge all into current branch" -- merge all task branches into the current branch in merge order
2. "Create a single combined branch" -- create a `kora/combined-<run-id>` branch with all task branches merged
3. "Leave branches as-is" -- do nothing, inform user of branch names

**New file:** `src/pipeline/merge.rs`

```rust
pub enum MergeStrategy {
    MergeIntoCurrent,
    CombinedBranch,
    LeaveAsIs,
}

pub async fn run_merge_flow(
    worktree_manager: &WorktreeManager,
    task_states: &HashMap<String, TaskState>,
    merge_order: &[String],
    run_id: &str,
    renderer: &mut Renderer,
) -> Result<MergeStrategy>
```

Uses `terminal::selector::select()` for the arrow-key picker.

### Task 4: Conflict Resolution During Merge

During the merge flow, if merging a branch causes a conflict:

1. Abort the failed merge
2. Show the user which files conflict
3. Respawn an implementor agent with:
   - The conflict files and both sides
   - The plan context for the conflicting task
   - Instructions to resolve and verify tests
4. After resolution, retry the merge
5. If resolution fails twice, skip the branch and inform the user

**Integrated into `merge.rs`:**
```rust
async fn merge_with_conflict_resolution(
    worktree_manager: &WorktreeManager,
    target_dir: &Path,
    branch_name: &str,
    task_id: &str,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    config: &Config,
    renderer: &mut Renderer,
) -> Result<MergeResult>
```

### Task 5: `kora resume` Command

**What it does:**
1. Scan `runs_dir` for interrupted runs (status is not Complete or Failed) using existing `RunDirectory::list_interrupted()`
2. If none found: print "no interrupted runs"
3. If one found: confirm with user, then resume
4. If multiple: use arrow-key selector to pick one
5. Load the `RunState` and determine the current stage
6. Resume the pipeline from that exact stage by calling into the orchestrator with a `resume_from` parameter

**Changes:**
- `PipelineOptions` gets a new field: `resume_run_id: Option<String>`
- `run_pipeline()` gets a new code path: if `resume_run_id` is set, load the existing RunState and skip to the appropriate stage
- Map each `Stage` to its resume entry point:
  - `Researching` -> start from researcher
  - `Reviewing` / `SecurityAuditing` / `Judging` -> restart current review loop iteration
  - `Planning` -> start from planner
  - `TestArchitecting` -> start from test architect
  - `Implementing` -> restart implementation (tasks already completed are skipped based on TASK_RESULT.md files)
  - `Validating` / `Fixing` -> restart validation loop
  - `AwaitingApproval(next)` -> show checkpoint prompt for `next`

**New file:** `src/cli/resume.rs`
**Changes:** `main.rs` wires the `Resume` command

### Task 6: `kora history` Command

**What it does:**
1. Scan `runs_dir` for all runs
2. Group by date (today, yesterday, older dates)
3. Display with status icon, request text (truncated), task count, duration
4. Arrow-key selector to pick a run for details
5. Detail view shows: request, stages completed, durations, errors

**New file:** `src/cli/history.rs`

```rust
pub fn run_history(project_root: &Path, config: &Config) -> Result<()>
```

### Task 7: `kora clean` Command

**What it does:**
1. Scan `runs_dir` for completed/failed runs
2. Calculate total disk usage
3. Arrow-key selector with options:
   - "All completed runs"
   - "Older than 1 week"
   - "Pick specific ones" (multi-select)
4. Also clean up stale kora worktrees via `WorktreeManager::cleanup_all()`
5. Print summary of what was cleaned

**New file:** `src/cli/clean.rs`

```rust
pub async fn run_clean(project_root: &Path, config: &Config) -> Result<()>
```

### Task 8: Inline Meta Commands

During the interactive session loop, before sending input to the pipeline, check for meta commands:

- `/status` -- show current run status or "no active run"
- `/config` -- print current config summary (provider, checkpoints, verbosity)
- `/verbose` -- cycle verbosity: focused -> detailed -> verbose -> focused
- `/help` -- list available commands

**New file:** `src/cli/meta_commands.rs`

```rust
pub enum MetaCommand {
    Status,
    Config,
    Verbose,
    Help,
    None(String),  // not a meta command, contains the original input
}

pub fn parse_meta_command(input: &str) -> MetaCommand
```

### Task 9: Wire Validator into Orchestrator

Modify `run_planning_and_implementation()` in `orchestrator.rs`:

After the implementation fleet loop, instead of directly going to `Complete`:
1. Merge all task branches into a temporary validation branch
2. Run the validation loop
3. If validation passes, run the merge flow
4. If merge flow succeeds, advance to `Complete`

The flow becomes: implementing -> validating -> (fixing loop) -> merge flow -> complete

### Task 10: Interactive Session Loop

Modify `main.rs` so the default `kora` command (no subcommand) runs a loop:

```
kora v0.1.0 · claude (default) · 2 checkpoints configured

ready. describe what you'd like to build, fix, or change.

> add dark mode support
  [pipeline runs...]
  all tasks complete.

ready for next request. type /help for commands, ctrl+c to exit.

> /status
  last run: "add dark mode support" · complete · 3m 12s

> fix the broken test in auth module
  [pipeline runs...]
```

The loop:
1. Print welcome (once)
2. Read input
3. Check for meta commands -> handle inline
4. If real input -> run pipeline
5. After pipeline completes -> print ready prompt
6. Repeat from step 2
7. Exit on Ctrl+C, empty input, or `/quit`

### Task 11: Tests

New test files:
- `tests/validation_test.rs` -- test `parse_validation()` edge cases, `build_validator_prompt()` context assembly
- `tests/merge_test.rs` -- test `MergeStrategy` enum, merge flow option presentation
- `tests/resume_test.rs` -- test interrupted run detection, stage-to-resume-point mapping
- `tests/history_test.rs` -- test run grouping by date, detail rendering
- `tests/clean_test.rs` -- test cleanup filtering (age, selection), disk usage calculation
- `tests/meta_commands_test.rs` -- test parsing of `/status`, `/config`, `/verbose`, `/help`, and non-commands

Update existing:
- `tests/state_test.rs` -- add tests for new stage transitions (Validating -> Fixing, etc.)
- `tests/pipeline_test.rs` -- add tests for merge flow option handling in pipeline options

## Implementation Order

1. Task 1 (validator agent) -- no dependencies
2. Task 8 (meta commands) -- no dependencies
3. Task 2 (validation loop) -- depends on Task 1
4. Task 3 (merge flow) -- no dependencies on validation
5. Task 4 (conflict resolution) -- depends on Task 3
6. Task 9 (wire into orchestrator) -- depends on Tasks 1, 2, 3, 4
7. Task 5 (resume) -- depends on Task 9 (needs to know the full pipeline shape)
8. Task 6 (history) -- no pipeline dependencies
9. Task 7 (clean) -- no pipeline dependencies
10. Task 10 (interactive session loop) -- depends on Tasks 8, 9
11. Task 11 (tests) -- after all implementation

## Files Created

- `src/pipeline/validation.rs`
- `src/pipeline/merge.rs`
- `src/cli/resume.rs`
- `src/cli/history.rs`
- `src/cli/clean.rs`
- `src/cli/meta_commands.rs`
- `tests/validation_test.rs`
- `tests/merge_test.rs`
- `tests/resume_test.rs`
- `tests/history_test.rs`
- `tests/clean_test.rs`
- `tests/meta_commands_test.rs`

## Files Modified

- `src/pipeline/mod.rs` -- add `pub mod validation;` and `pub mod merge;`
- `src/pipeline/context.rs` -- add `build_validator_prompt()`
- `src/pipeline/orchestrator.rs` -- add validation loop, merge flow, resume support
- `src/cli/mod.rs` -- add `pub mod resume;`, `pub mod history;`, `pub mod clean;`, `pub mod meta_commands;`
- `src/main.rs` -- implement resume/history/clean commands, interactive session loop
- `src/state/stage.rs` -- add `Merging` stage and transitions (if needed)
- `src/terminal/renderer.rs` -- add validation/merge rendering methods
- `tests/state_test.rs` -- new transition tests
- `tests/pipeline_test.rs` -- new pipeline option tests
