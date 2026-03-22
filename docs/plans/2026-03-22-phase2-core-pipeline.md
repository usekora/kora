# Kora Phase 2: Core Pipeline — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the core orchestration pipeline covering the research-review loop: researcher (interactive session), reviewer + security auditor (parallel), judge, and the revision cycle. Wire it into the CLI so `kora` and `kora run` start the pipeline. Handle checkpoints, review loop convergence, and failure modes.

**Spec:** `docs/specs/2026-03-22-kora-design.md`

**Depends on:** Phase 1 (complete) — CLI skeleton, config, state machine, providers, terminal UI, prompt loading, output parsers.

---

## File Structure (new and modified files)

```
kora/
├── prompts/                          ← populate with full prompts from spec
│   ├── researcher.md                 ← MODIFY (replace placeholder)
│   ├── reviewer.md                   ← MODIFY (replace placeholder)
│   ├── security_auditor.md           ← MODIFY (replace placeholder)
│   ├── judge.md                      ← MODIFY (replace placeholder)
│   ├── planner.md                    ← MODIFY (replace placeholder)
│   ├── test_architect.md             ← MODIFY (replace placeholder)
│   ├── implementor.md                ← MODIFY (replace placeholder)
│   └── validator.md                  ← MODIFY (replace placeholder)
├── src/
│   ├── main.rs                       ← MODIFY (wire pipeline into CLI commands)
│   ├── lib.rs                        ← MODIFY (add pipeline module)
│   ├── pipeline/
│   │   ├── mod.rs                    ← pipeline module exports
│   │   ├── orchestrator.rs           ← main pipeline orchestrator
│   │   ├── review_loop.rs            ← review loop logic (reviewer + security + judge cycle)
│   │   ├── researcher.rs             ← researcher session management (interactive + revision)
│   │   └── context.rs                ← prompt context assembly for each agent
│   ├── agent/
│   │   ├── prompts.rs                ← MODIFY (add revision prompt assembly helper)
│   │   └── output_parser.rs          ← MODIFY (add parse_security_review, extract_plan)
│   ├── state/
│   │   ├── run.rs                    ← MODIFY (add set_error, increment_iteration helpers)
│   │   └── stage.rs                  ← MODIFY (add checkpoint_for_stage helper)
│   └── terminal/
│       └── renderer.rs               ← MODIFY (add checkpoint_prompt, review_loop_summary)
├── tests/
│   ├── pipeline_test.rs              ← pipeline orchestration unit tests
│   ├── review_loop_test.rs           ← review loop state machine tests
│   ├── context_test.rs               ← prompt context assembly tests
│   └── output_parser_test.rs         ← MODIFY (add security review + plan extraction tests)
```

---

### Task 1: Populate All 8 Prompt Files

Replace every placeholder in `prompts/` with the full prompt text from the design spec.

**Files:**
- Modify: `prompts/researcher.md`
- Modify: `prompts/reviewer.md`
- Modify: `prompts/security_auditor.md`
- Modify: `prompts/judge.md`
- Modify: `prompts/planner.md`
- Modify: `prompts/test_architect.md`
- Modify: `prompts/implementor.md`
- Modify: `prompts/validator.md`

Copy each prompt verbatim from the spec's "Base Prompt" sections.

**Verification:**
- `cargo build` succeeds (prompts are compiled via `include_str!`)
- Existing tests still pass

---

### Task 2: Extend Output Parser

Add `parse_security_review` (parses `<!-- SECURITY -->` blocks) and `extract_plan` (extracts `<!-- PLAN -->` blocks) to the output parser.

**Files:**
- Modify: `src/agent/output_parser.rs`
- Modify: `tests/output_parser_test.rs`

**Implementation:**

```rust
// In output_parser.rs — add:

pub fn parse_security_review(text: &str) -> Option<ReviewSummary> {
    let block = extract_block(text, "<!-- SECURITY -->", "<!-- /SECURITY -->")?;
    // Same parsing logic as parse_review — reuse internal helper
    parse_findings_block(&block)
}

pub fn extract_plan(text: &str) -> Option<String> {
    extract_block(text, "<!-- PLAN -->", "<!-- /PLAN -->")
}
```

Refactor `parse_review` to share a `parse_findings_block` helper with `parse_security_review`.

**Tests:**
- `test_parse_security_review_summary` — parses security findings block
- `test_parse_security_review_missing_markers` — returns None
- `test_extract_plan` — extracts plan content between markers
- `test_extract_plan_missing_markers` — returns None

---

### Task 3: Extend State Helpers

Add utility methods to `RunState` and a checkpoint-matching helper to `stage.rs`.

**Files:**
- Modify: `src/state/run.rs`
- Modify: `src/state/stage.rs`

**Implementation:**

```rust
// In run.rs — add:
impl RunState {
    pub fn set_error(&mut self, err: &str) {
        self.error = Some(err.to_string());
        self.status = Stage::Failed(err.to_string());
        self.updated_at = Utc::now();
    }

    pub fn increment_iteration(&mut self) {
        self.current_iteration += 1;
        self.updated_at = Utc::now();
    }
}

// In stage.rs — add:
pub fn checkpoint_for_stage(stage: &Stage, checkpoints: &[Checkpoint]) -> Option<Checkpoint> {
    match stage {
        Stage::Reviewing => {
            if checkpoints.contains(&Checkpoint::AfterResearcher) {
                Some(Checkpoint::AfterResearcher)
            } else {
                None
            }
        }
        Stage::Judging => {
            if checkpoints.contains(&Checkpoint::AfterReviewLoop) {
                Some(Checkpoint::AfterReviewLoop)
            } else {
                None
            }
        }
        _ => None,
    }
}
```

---

### Task 4: Extend Terminal Renderer

Add methods for checkpoint approval prompts and review loop summary display.

**Files:**
- Modify: `src/terminal/renderer.rs`

**Implementation:**

```rust
impl Renderer {
    pub fn checkpoint_prompt(&mut self, next_stage: &str) -> bool {
        // Display: "checkpoint: approve to proceed to {next_stage}?"
        // Read y/n from stdin
        // Returns true if approved
    }

    pub fn review_loop_summary(&mut self, iteration: u32, valid: u32, dismissed: u32, overall: &str) {
        // Display: "review loop iteration {N}: {valid} valid, {dismissed} dismissed -> {overall}"
    }

    pub fn escalation(&mut self, message: &str) {
        // Display escalation message for max iterations exceeded
    }

    pub fn iteration_header(&mut self, iteration: u32, max: u32) {
        // Display: "review loop ··· iteration {N} of {max}"
    }
}
```

---

### Task 5: Build Prompt Context Assembly

Create `src/pipeline/context.rs` — responsible for assembling the full prompt for each agent by reading files from the run directory and combining with base prompts.

**Files:**
- Create: `src/pipeline/context.rs`
- Create: `tests/context_test.rs`

**Implementation:**

```rust
use std::path::Path;
use anyhow::Result;
use crate::agent::prompts;
use crate::config::AgentConfig;

pub struct PromptContext {
    pub prompt: String,
}

pub fn build_researcher_prompt(
    run_dir: &Path,
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
    custom_instructions: Option<&str>,
) -> Result<PromptContext> {
    // Read: context/researcher-plan.md, reviews/iteration-N/judgment.md
    // Build revision context with current plan + valid findings
}

pub fn build_reviewer_prompt(
    run_dir: &Path,
    iteration: u32,
    custom_instructions: Option<&str>,
) -> Result<PromptContext> {
    // Read: context/researcher-plan.md, context/codebase-summary.md
    // If iteration > 1: also read previous reviews + judgments
}

pub fn build_security_prompt(
    run_dir: &Path,
    iteration: u32,
    custom_instructions: Option<&str>,
) -> Result<PromptContext> {
    // Same as reviewer but with security auditor base prompt
}

pub fn build_judge_prompt(
    run_dir: &Path,
    iteration: u32,
    custom_instructions: Option<&str>,
) -> Result<PromptContext> {
    // Read: context/researcher-plan.md, reviews/iteration-N/review.md,
    //        reviews/iteration-N/security-audit.md
    // If iteration > 1: also read previous judgments
}
```

**Tests (in `tests/context_test.rs`):**
- `test_build_researcher_prompt_includes_request` — verify request text appears in assembled prompt
- `test_build_researcher_prompt_includes_base_prompt` — verify base prompt is included
- `test_build_reviewer_prompt_includes_plan` — set up a temp run dir with researcher-plan.md, verify it's in the prompt
- `test_build_judge_prompt_includes_review_and_security` — set up review + security files, verify both in prompt
- `test_build_researcher_revision_prompt_includes_findings` — set up judgment file, verify valid findings in prompt

---

### Task 6: Build Researcher Session Manager

Create `src/pipeline/researcher.rs` — manages the researcher's interactive session and non-interactive revision mode.

**Files:**
- Create: `src/pipeline/researcher.rs`

**Implementation:**

```rust
use std::path::{Path, PathBuf};
use anyhow::Result;
use crate::provider::Provider;
use crate::agent::output_parser;

pub struct ResearcherResult {
    pub plan_path: PathBuf,
    pub summary_path: Option<PathBuf>,
}

pub async fn run_interactive(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &Path,
) -> Result<ResearcherResult> {
    // 1. Spawn interactive session via provider.run_interactive()
    // 2. Wait for process exit
    // 3. Check for context/researcher-plan.md in working_dir
    // 4. If not found, try extracting from stdout via <!-- PLAN --> markers
    // 5. Copy plan file to run_dir/context/researcher-plan.md
    // 6. Return paths
}

pub async fn run_revision(
    provider: &dyn Provider,
    prompt: &str,
    working_dir: &Path,
    run_dir: &Path,
) -> Result<ResearcherResult> {
    // 1. Run non-interactive via provider.run()
    // 2. Extract revised plan from output (file or markers)
    // 3. Overwrite run_dir/context/researcher-plan.md
    // 4. Return paths
}
```

---

### Task 7: Build the Review Loop

Create `src/pipeline/review_loop.rs` — manages the reviewer + security auditor (parallel) -> judge -> revision cycle.

**Files:**
- Create: `src/pipeline/review_loop.rs`
- Create: `tests/review_loop_test.rs`

**Implementation:**

```rust
use anyhow::Result;
use std::path::Path;
use crate::config::Config;
use crate::provider::Provider;
use crate::state::{RunState, RunDirectory, Stage};
use crate::agent::output_parser::{self, Verdict};
use crate::terminal::Renderer;

#[derive(Debug, PartialEq)]
pub enum ReviewOutcome {
    Approved,
    Escalated { iteration: u32, reason: String },
}

pub async fn run_review_loop(
    config: &Config,
    run_state: &mut RunState,
    run_dir: &RunDirectory,
    project_root: &Path,
    renderer: &mut Renderer,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
) -> Result<ReviewOutcome> {
    let max = config.review_loop.max_iterations;

    for iteration in 1..=max {
        run_state.increment_iteration();
        renderer.iteration_header(iteration, max);

        // 1. Run reviewer + security auditor in parallel
        run_state.advance(Stage::Reviewing);
        run_state.save(&config.runs_dir)?; // not exact — need project_root prefix

        let (review_output, security_output) = run_parallel_reviews(
            config, run_dir, iteration, project_root, get_provider,
        ).await?;

        // 2. Save outputs to run directory
        save_review_outputs(run_dir, iteration, &review_output, &security_output)?;

        // 3. Run judge
        run_state.advance(Stage::Judging);
        let verdict = run_judge(config, run_dir, iteration, project_root, get_provider).await?;

        // 4. Save judgment
        save_judgment(run_dir, iteration, &verdict)?;

        // 5. Render summary
        renderer.review_loop_summary(iteration, verdict.valid_count, verdict.dismissed_count, &verdict.overall);

        // 6. Check verdict
        if verdict.overall == "APPROVE" {
            return Ok(ReviewOutcome::Approved);
        }

        // 7. If REVISE and not last iteration, run researcher revision
        if iteration < max {
            run_state.advance(Stage::Researching);
            run_researcher_revision(config, run_dir, project_root, get_provider).await?;
        }
    }

    Ok(ReviewOutcome::Escalated {
        iteration: max,
        reason: format!("Review loop did not converge after {} iterations", max),
    })
}
```

**Tests (in `tests/review_loop_test.rs`):**
- `test_review_outcome_approved_serialization` — verify enum variants
- `test_review_loop_max_iterations_boundary` — verify escalation after max

---

### Task 8: Build the Pipeline Orchestrator

Create `src/pipeline/orchestrator.rs` — the main entry point that drives the full pipeline from start through research-review loop.

**Files:**
- Create: `src/pipeline/orchestrator.rs`
- Create: `src/pipeline/mod.rs`
- Create: `tests/pipeline_test.rs`

**Implementation:**

```rust
use anyhow::Result;
use std::path::Path;
use crate::config::Config;
use crate::provider::{self, Provider};
use crate::state::{RunState, RunDirectory, Stage, Checkpoint, checkpoint_for_stage};
use crate::terminal::Renderer;
use crate::pipeline::{review_loop, researcher, context};

pub struct PipelineOptions {
    pub request: String,
    pub yolo: bool,
    pub careful: bool,
    pub dry_run: bool,
    pub provider_override: Option<String>,
}

pub async fn run_pipeline(
    config: &Config,
    project_root: &Path,
    options: PipelineOptions,
    renderer: &mut Renderer,
) -> Result<()> {
    // 1. Create run state and directory
    let mut run_state = RunState::new(&options.request);
    let runs_dir = project_root.join(&config.runs_dir);
    let run_dir = RunDirectory::new(&runs_dir, &run_state.id);
    run_dir.create_structure()?;
    run_state.save(&runs_dir)?;

    // 2. Determine effective checkpoints
    let checkpoints = effective_checkpoints(config, &options);

    // 3. Build provider resolver
    let get_provider = |agent_provider: &str| -> Option<Box<dyn Provider>> {
        let effective = options.provider_override.as_deref().unwrap_or(agent_provider);
        provider::create_provider(config, effective)
    };

    // 4. Run researcher (interactive)
    renderer.stage_header("researcher", "starting");
    let researcher_prompt = context::build_researcher_prompt(
        run_dir.context_dir().as_path(),
        &options.request,
        load_custom_instructions(project_root, &config.agents.researcher).as_deref(),
    )?;
    researcher::run_interactive(
        get_provider(&config.agents.researcher.provider).as_ref().unwrap().as_ref(),
        &researcher_prompt.prompt,
        project_root,
        &run_dir,
    ).await?;
    renderer.stage_complete("researcher", 0);

    // 5. Checkpoint after researcher
    if should_checkpoint(&Stage::Reviewing, &checkpoints) {
        if !renderer.checkpoint_prompt("review loop") {
            run_state.set_error("User declined at researcher checkpoint");
            run_state.save(&runs_dir)?;
            return Ok(());
        }
    }

    // 6. Run review loop
    let outcome = review_loop::run_review_loop(
        config, &mut run_state, &run_dir, project_root, renderer, &get_provider,
    ).await?;

    match outcome {
        review_loop::ReviewOutcome::Approved => {
            renderer.info("plan approved — ready for planning phase");
            if options.dry_run {
                renderer.info("dry run mode — stopping after review loop");
                run_state.advance(Stage::Complete);
                run_state.save(&runs_dir)?;
                return Ok(());
            }
            // Phase 3 will continue from here (planner -> implementors -> validator)
            renderer.info("implementation pipeline not yet built — coming in Phase 3");
        }
        review_loop::ReviewOutcome::Escalated { iteration, reason } => {
            renderer.escalation(&format!("Review loop escalated after {} iterations: {}", iteration, reason));
            // Pause for user decision
        }
    }

    run_state.save(&runs_dir)?;
    Ok(())
}

fn effective_checkpoints(config: &Config, options: &PipelineOptions) -> Vec<Checkpoint> {
    if options.yolo {
        vec![]
    } else if options.careful {
        vec![
            Checkpoint::AfterResearcher,
            Checkpoint::AfterReviewLoop,
            Checkpoint::AfterPlanner,
            Checkpoint::AfterImplementation,
        ]
    } else {
        config.checkpoints.clone()
    }
}

fn should_checkpoint(next_stage: &Stage, checkpoints: &[Checkpoint]) -> bool {
    checkpoint_for_stage(next_stage, checkpoints).is_some()
}
```

**Tests (in `tests/pipeline_test.rs`):**
- `test_effective_checkpoints_yolo_is_empty`
- `test_effective_checkpoints_careful_has_all`
- `test_effective_checkpoints_default_uses_config`

---

### Task 9: Wire Pipeline into CLI

Modify `src/main.rs` to call the pipeline orchestrator for both the default command and `kora run`.

**Files:**
- Modify: `src/main.rs`
- Modify: `src/lib.rs`

**Implementation:**
- Add `pub mod pipeline;` to `lib.rs`
- In `main.rs`, replace the placeholder `println!` in both `None` (default) and `Commands::Run` arms with calls to `pipeline::orchestrator::run_pipeline()`
- Use `tokio::runtime::Runtime` to run the async pipeline from the sync main

---

### Task 10: Integration — Build, Test, Lint

- [ ] `cargo build` — fix all compilation errors
- [ ] `cargo test` — fix all test failures (existing + new)
- [ ] `cargo clippy -- -D warnings` — fix all warnings
- [ ] Do NOT commit

---

## Summary

Phase 2 delivers:
1. Full agent prompts (8 files) baked into the binary
2. Pipeline orchestrator that drives research -> review loop
3. Review loop with parallel reviewer + security auditor, judge evaluation, and researcher revision
4. Interactive researcher session with plan file detection and fallback extraction
5. Non-interactive agent execution for reviewer, security auditor, judge
6. Checkpoint system (yolo / careful / configured)
7. Review loop convergence with max iterations and escalation
8. Prompt context assembly reading from the run directory
9. CLI wired to start the pipeline

Phase 3 (future) will add: planner, test architect, implementor fleet, validator, merge flow.
