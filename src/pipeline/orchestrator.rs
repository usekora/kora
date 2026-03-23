use anyhow::{Context as AnyhowContext, Result};
use std::path::Path;
use std::time::Duration;

use crate::agent::output_parser;
use crate::config::Config;
use crate::git::worktree::WorktreeManager;
use crate::pipeline::{
    context, implementation, merge, metrics::RunMetrics, planner, researcher, review_loop, stall,
    test_architect, validation,
};
use crate::provider::{self, Provider};
use crate::state::{
    checkpoint_for_stage, Checkpoint, PipelineProfile, RunDirectory, RunState, Stage,
};
use crate::terminal::dashboard::Dashboard;
use crate::terminal::Renderer;

pub struct PipelineOptions {
    pub request: String,
    pub yolo: bool,
    pub careful: bool,
    pub dry_run: bool,
    pub provider_override: Option<String>,
    pub resume_run_id: Option<String>,
    pub profile_override: Option<PipelineProfile>,
}

pub async fn run_pipeline(
    config: &Config,
    project_root: &Path,
    options: PipelineOptions,
    renderer: &mut Renderer,
) -> Result<()> {
    let runs_dir = crate::config::runs_dir();
    let checkpoints = effective_checkpoints(config, &options);

    let get_provider = |agent_provider: &str| -> Option<Box<dyn Provider>> {
        if let Some(ref override_name) = options.provider_override {
            provider::create_provider(config, override_name)
        } else {
            provider::create_provider(config, agent_provider)
        }
    };

    if let Some(ref resume_id) = options.resume_run_id {
        let mut run_state = RunState::load(&runs_dir, resume_id)?;
        let run_dir = RunDirectory::new(&runs_dir, &run_state.id);

        let effective_config =
            apply_profile_to_config(config, run_state.pipeline_profile.unwrap_or_default());

        resume_pipeline(
            &effective_config,
            &mut run_state,
            &run_dir,
            &runs_dir,
            project_root,
            renderer,
            &checkpoints,
            &get_provider,
            &options,
        )
        .await?;

        run_state.save(&runs_dir)?;
        return Ok(());
    }

    let mut run_state = RunState::new(&options.request);
    let run_dir = RunDirectory::new(&runs_dir, &run_state.id);
    run_dir.create_structure()?;
    run_state.save(&runs_dir)?;

    let mut metrics = RunMetrics::new();

    let _spinner = renderer.stage_header("researcher", "starting");

    let researcher_prompt = context::build_researcher_prompt(
        &options.request,
        load_custom_instructions(project_root, &config.agents.researcher).as_deref(),
    )?;

    let researcher_provider = get_provider(&config.agents.researcher.provider);
    match researcher_provider {
        Some(p) => {
            let no_flags: Vec<String> = vec![];
            researcher::run_interactive(
                p.as_ref(),
                &researcher_prompt.prompt,
                project_root,
                &run_dir,
                &no_flags,
            )
            .await?;
        }
        None => {
            run_state.set_error("no provider available for researcher");
            run_state.save(&runs_dir)?;
            renderer.escalation("no provider available for researcher");
            return Ok(());
        }
    }

    drop(_spinner);

    renderer.stage_complete("researcher", 0);

    // Determine pipeline profile: CLI override > researcher classification > Standard
    let profile = determine_pipeline_profile(&options, &run_dir);
    run_state.pipeline_profile = Some(profile);
    run_state.save(&runs_dir)?;
    renderer.info(&format!("pipeline profile: {}", profile));

    // Build effective config with agent flags adjusted for the profile
    let effective_config = apply_profile_to_config(config, profile);

    if options.dry_run {
        renderer.info("dry run mode -- stopping after research");
        run_state.advance(Stage::Complete);
        run_state.save(&runs_dir)?;
        return Ok(());
    }

    // Review loop — only for Standard and SecurityCritical profiles
    if profile.has_review_loop() {
        if should_checkpoint(&Stage::Reviewing, &checkpoints)
            && !renderer.checkpoint_prompt("review loop")
        {
            run_state.set_error("user declined at researcher checkpoint");
            run_state.save(&runs_dir)?;
            renderer.info("run cancelled by user at researcher checkpoint");
            return Ok(());
        }

        let outcome = review_loop::run_review_loop(
            &effective_config,
            &mut run_state,
            &run_dir,
            &runs_dir,
            project_root,
            renderer,
            &get_provider,
        )
        .await?;

        match outcome {
            review_loop::ReviewOutcome::Approved => {
                renderer.info("plan approved by review loop");
            }
            review_loop::ReviewOutcome::Escalated { iteration, reason } => {
                renderer.escalation(&format!(
                    "review loop escalated after {} iterations: {}",
                    iteration, reason
                ));
                run_state.set_error(&reason);
                run_state.save(&runs_dir)?;
                return Ok(());
            }
        }
    } else {
        renderer.info(&format!("skipping review loop (profile: {})", profile));
    }

    if should_checkpoint(&Stage::Planning, &checkpoints) && !renderer.checkpoint_prompt("planning")
    {
        run_state.set_error("user declined at review loop checkpoint");
        run_state.save(&runs_dir)?;
        renderer.info("run cancelled by user at review loop checkpoint");
        return Ok(());
    }

    run_planning_and_implementation(
        &effective_config,
        &mut run_state,
        &run_dir,
        &runs_dir,
        project_root,
        renderer,
        &checkpoints,
        &get_provider,
        options.yolo,
        &mut metrics,
    )
    .await?;

    // Finalize and display metrics
    metrics.complete();
    let run_dir_path = runs_dir.join(&run_state.id);
    if let Err(e) = metrics.save(&run_dir_path) {
        renderer.info(&format!("warning: failed to save metrics: {}", e));
    }
    renderer.run_metrics_summary(&metrics.summary_lines());

    run_state.save(&runs_dir)?;

    let worktree_manager = WorktreeManager::new(project_root);
    if let Err(e) = worktree_manager.cleanup_all().await {
        renderer.info(&format!("warning: worktree cleanup failed: {}", e));
    }

    Ok(())
}

fn determine_pipeline_profile(
    options: &PipelineOptions,
    run_dir: &RunDirectory,
) -> PipelineProfile {
    // CLI override takes precedence
    if let Some(profile) = options.profile_override {
        return profile;
    }

    // Try to parse from researcher output
    let plan_path = run_dir.context_dir().join("researcher-plan.md");
    if let Ok(content) = std::fs::read_to_string(&plan_path) {
        if let Some(profile) = output_parser::parse_classification(&content) {
            return profile;
        }
    }

    // Default to Standard
    PipelineProfile::Standard
}

/// Clone config and adjust agent enabled flags based on the pipeline profile.
fn apply_profile_to_config(config: &Config, profile: PipelineProfile) -> Config {
    let mut c = config.clone();
    match profile {
        PipelineProfile::Trivial => {
            c.agents.plan_reviewer.enabled = false;
            c.agents.plan_security_auditor.enabled = false;
            c.agents.judge.enabled = false;
            c.agents.test_architect.enabled = false;
            c.agents.code_reviewer.enabled = false;
            c.agents.code_security_auditor.enabled = false;
            c.agents.validator.enabled = false;
        }
        PipelineProfile::Simple => {
            c.agents.plan_reviewer.enabled = false;
            c.agents.plan_security_auditor.enabled = false;
            c.agents.judge.enabled = false;
            c.agents.test_architect.enabled = false;
            c.agents.code_security_auditor.enabled = false;
        }
        PipelineProfile::Standard => {
            // Keep user's config as-is
        }
        PipelineProfile::SecurityCritical => {
            // Force-enable all security agents regardless of user config
            c.agents.plan_reviewer.enabled = true;
            c.agents.plan_security_auditor.enabled = true;
            c.agents.judge.enabled = true;
            c.agents.test_architect.enabled = true;
            c.agents.code_reviewer.enabled = true;
            c.agents.code_security_auditor.enabled = true;
            c.agents.validator.enabled = true;
        }
    }
    c
}

fn synthetic_trivial_breakdown(request: &str) -> output_parser::TaskBreakdown {
    let title: String = request.chars().take(80).collect();
    output_parser::TaskBreakdown {
        tasks: vec![output_parser::Task {
            id: "task-1".to_string(),
            title,
            description: request.to_string(),
            files: output_parser::TaskFiles {
                create: vec![],
                modify: vec![],
                delete: vec![],
            },
            depends_on: vec![],
            estimated_complexity: "trivial".to_string(),
            conflict_risk: vec![],
        }],
        branch_strategy: "single".to_string(),
        merge_order: vec!["task-1".to_string()],
        critical_path: vec!["task-1".to_string()],
        parallelism_summary: "1 task, no parallelism".to_string(),
    }
}

fn empty_test_strategy() -> output_parser::TestStrategy {
    output_parser::TestStrategy {
        per_task: std::collections::HashMap::new(),
        post_merge: output_parser::PostMergeTests {
            integration_tests: vec![],
        },
        testing_patterns: output_parser::TestingPatterns {
            framework: String::new(),
            conventions: String::new(),
        },
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_planning_and_implementation(
    config: &Config,
    run_state: &mut RunState,
    run_dir: &RunDirectory,
    runs_dir: &Path,
    project_root: &Path,
    renderer: &mut Renderer,
    checkpoints: &[Checkpoint],
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    auto_merge: bool,
    metrics: &mut RunMetrics,
) -> Result<()> {
    let no_flags: Vec<String> = vec![];
    let profile = run_state.pipeline_profile.unwrap_or_default();

    // Planner — skip for Trivial, use synthetic single-task breakdown
    run_state.advance(Stage::Planning);
    run_state.save(runs_dir)?;

    let breakdown = if profile.has_planner() {
        let _spinner = renderer.stage_header("planner", "decomposing");

        let planner_provider =
            get_provider(&config.agents.planner.provider).context("no provider for planner")?;
        let planner_prompt = context::build_planner_prompt(
            &runs_dir.join(&run_state.id),
            &run_state.request,
            project_root,
            config.agents.planner.custom_instructions.as_deref(),
        )?;

        let bd = planner::run_planner(
            planner_provider.as_ref(),
            &planner_prompt.prompt,
            project_root,
            run_dir,
            &no_flags,
            config.agents.planner.timeout_seconds,
        )
        .await?;

        // Record planner metrics from saved output
        let planner_output_path = run_dir.plan_dir().join("breakdown.json");
        let planner_output = std::fs::read_to_string(&planner_output_path).unwrap_or_default();
        metrics.record(
            "planner",
            &config.agents.planner.provider,
            std::time::Duration::from_secs(0),
            &planner_prompt.prompt,
            &planner_output,
        );

        drop(_spinner);


        renderer.stage_complete("planner", 0);
        renderer.info(&format!(
            "{} tasks, strategy: {}, critical path: {}",
            bd.tasks.len(),
            bd.branch_strategy,
            bd.critical_path.join(" -> "),
        ));
        bd
    } else {
        renderer.info("skipping planner (trivial profile), using single-task breakdown");
        synthetic_trivial_breakdown(&run_state.request)
    };

    // Test Architect — skip for Trivial and Simple
    let test_strategy = if config.agents.test_architect.enabled && profile.has_test_architect() {
        let _spinner = renderer.stage_header("test architect", "designing tests");
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

        let strategy = test_architect::run_test_architect(
            ta_provider.as_ref(),
            &ta_prompt.prompt,
            project_root,
            run_dir,
            &no_flags,
            config.agents.test_architect.timeout_seconds,
        )
        .await?;

        drop(_spinner);


        renderer.stage_complete("test architect", 0);
        strategy
    } else {
        if !profile.has_test_architect() {
            renderer.info("skipping test architect (profile)");
        } else {
            renderer.info("test architect disabled, skipping test planning");
        }
        run_state.advance(Stage::TestArchitecting);
        run_state.save(runs_dir)?;
        empty_test_strategy()
    };

    if should_checkpoint(&Stage::Implementing, checkpoints)
        && !renderer.checkpoint_prompt("implementation")
    {
        run_state.set_error("user declined at planner checkpoint");
        run_state.save(runs_dir)?;
        renderer.info("run cancelled by user at planner checkpoint");
        return Ok(());
    }

    run_state.advance(Stage::Implementing);
    run_state.save(runs_dir)?;

    let mut fleet = implementation::ImplementationFleet::new(
        config.clone(),
        breakdown.clone(),
        test_strategy,
        project_root,
        run_dir,
    );

    let task_order: Vec<String> = breakdown.merge_order.clone();
    let mut dashboard = Dashboard::new(task_order);

    let (tx, mut rx) = tokio::sync::mpsc::channel::<implementation::TaskEvent>(64);
    let parallel_limit = config.implementation.parallel_limit as usize;

    loop {
        fleet.check_unblocked();

        let available_slots = parallel_limit.saturating_sub(fleet.running_count());
        let mut ready = fleet.ready_tasks();
        ready.truncate(available_slots);

        for task_id in &ready {
            fleet.spawn_task(task_id, get_provider, tx.clone()).await?;
        }

        dashboard.render(fleet.task_states());

        if fleet.is_done() {
            break;
        }

        while let Ok(event) = rx.try_recv() {
            fleet.handle_event(&event);
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        while let Ok(event) = rx.try_recv() {
            fleet.handle_event(&event);
        }
    }

    let failed = fleet.failed_tasks();

    if !failed.is_empty() {
        for task_id in &failed {
            if let Some(state) = fleet.task_states().get(task_id) {
                if let implementation::ImplementationTaskStatus::Failed { ref error, .. } =
                    state.status
                {
                    renderer.task_failure(task_id, error);
                }
            }
        }
        run_state.set_error(&format!("tasks failed: {}", failed.join(", ")));
        return Ok(());
    }

    renderer.implementation_complete(fleet.total_tasks(), 0);

    run_validation_and_merge(
        config,
        run_state,
        run_dir,
        runs_dir,
        project_root,
        renderer,
        get_provider,
        fleet.task_states(),
        fleet.merge_order(),
        auto_merge,
    )
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_validation_and_merge(
    config: &Config,
    run_state: &mut RunState,
    run_dir: &RunDirectory,
    runs_dir: &Path,
    project_root: &Path,
    renderer: &mut Renderer,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    task_states: &std::collections::HashMap<String, implementation::TaskState>,
    merge_order: &[String],
    auto_merge: bool,
) -> Result<()> {
    let no_flags: Vec<String> = vec![];

    if !config.agents.validator.enabled {
        renderer.info("validator disabled, skipping validation");
    } else {
        let max_iterations = config.validation_loop.max_iterations;
        let mut previous_validator_output = String::new();

        for iteration in 1..=max_iterations {
            run_state.advance(Stage::Validating);
            run_state.save(runs_dir)?;

            let _spinner = renderer.stage_header("validator", &format!("iteration {}", iteration));

            let validator_provider = get_provider(&config.agents.validator.provider)
                .context("no provider for validator")?;
            let validator_prompt = context::build_validator_prompt(
                &runs_dir.join(&run_state.id),
                &run_state.request,
                project_root,
                config.agents.validator.custom_instructions.as_deref(),
            )?;

            let validator_output = validation::run_validator(
                validator_provider.as_ref(),
                &validator_prompt.prompt,
                project_root,
                run_dir,
                &no_flags,
                config.agents.validator.timeout_seconds,
            )
            .await?;

            drop(_spinner);


            renderer.stage_complete("validator", 0);
            renderer.info(&format!(
                "validation: {} blocking, {} minor, tests: {} passed / {} failed",
                validator_output.validation.blocking_issues,
                validator_output.validation.minor_issues,
                validator_output.validation.tests_passed,
                validator_output.validation.tests_failed,
            ));

            // Stall detection: break if validator output is cycling
            if iteration > 1
                && stall::is_cycling(
                    &previous_validator_output,
                    &validator_output.raw_text,
                    stall::DEFAULT_CYCLE_THRESHOLD,
                )
            {
                renderer.cycling_detected("validation loop");
                run_state.set_error("validation loop cycling — same issues repeating");
                run_state.save(runs_dir)?;
                return Ok(());
            }
            previous_validator_output = validator_output.raw_text.clone();

            if validator_output.validation.passed
                || (validator_output.validation.blocking_issues == 0
                    && validator_output.validation.tests_failed == 0)
            {
                renderer.info("validation passed");
                break;
            }

            if iteration >= max_iterations {
                renderer.escalation(&format!(
                    "validation failed after {} iterations",
                    max_iterations
                ));
                run_state.set_error("validation loop exceeded max iterations");
                run_state.save(runs_dir)?;
                return Ok(());
            }

            run_state.advance(Stage::Fixing);
            run_state.save(runs_dir)?;

            let _spinner = renderer.stage_header("fixer", "applying fixes");

            let fixes = validation::extract_required_fixes(&validator_output.raw_text);

            if fixes.is_empty() {
                renderer.escalation("validator reported FAIL but no required fixes were found");
                run_state.set_error("validation failed with no actionable fixes");
                run_state.save(runs_dir)?;
                return Ok(());
            }

            let fix_provider = get_provider(&config.agents.implementor.provider)
                .context("no provider for fixer")?;

            let fix_prompt = context::build_fix_prompt(
                &runs_dir.join(&run_state.id),
                &run_state.request,
                &fixes,
                project_root,
                config.agents.implementor.custom_instructions.as_deref(),
            )?;

            let fix_timeout = Some(Duration::from_secs(
                config.agents.implementor.timeout_seconds,
            ));
            let fix_output = fix_provider
                .run(&fix_prompt.prompt, project_root, &no_flags, fix_timeout)
                .await
                .context("fixer agent failed")?;

            let fix_dir = run_dir.validation_dir().join(format!("fix-{}", iteration));
            std::fs::create_dir_all(&fix_dir)?;
            std::fs::write(fix_dir.join("output.md"), &fix_output.text)?;

            drop(_spinner);


            renderer.stage_complete("fixer", fix_output.duration.as_secs());
        }
    } // end validator enabled block

    let worktree_manager = WorktreeManager::new(project_root);
    let merge_outcome = merge::run_merge_flow(
        &worktree_manager,
        task_states,
        merge_order,
        &run_state.id,
        config,
        renderer,
        get_provider,
        auto_merge,
    )
    .await?;

    if !merge_outcome.failed_branches.is_empty() {
        renderer.info(&format!(
            "warning: {} branches failed to merge",
            merge_outcome.failed_branches.len()
        ));
    }

    run_state.advance(Stage::Complete);
    run_state.save(runs_dir)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn resume_pipeline(
    config: &Config,
    run_state: &mut RunState,
    run_dir: &RunDirectory,
    runs_dir: &Path,
    project_root: &Path,
    renderer: &mut Renderer,
    checkpoints: &[Checkpoint],
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
    options: &PipelineOptions,
) -> Result<()> {
    let stage = run_state.status.clone();
    renderer.info(&format!("resuming from stage: {}", stage.label()));

    let mut metrics = RunMetrics::new();

    match stage {
        Stage::Researching => {
            let _spinner = renderer.stage_header("researcher", "resuming");
            let researcher_prompt = context::build_researcher_prompt(
                &run_state.request,
                load_custom_instructions(project_root, &config.agents.researcher).as_deref(),
            )?;

            let researcher_provider = get_provider(&config.agents.researcher.provider)
                .context("no provider for researcher")?;
            let no_flags: Vec<String> = vec![];
            researcher::run_interactive(
                researcher_provider.as_ref(),
                &researcher_prompt.prompt,
                project_root,
                run_dir,
                &no_flags,
            )
            .await?;
            drop(_spinner);

            renderer.stage_complete("researcher", 0);

            let outcome = review_loop::run_review_loop(
                config,
                run_state,
                run_dir,
                runs_dir,
                project_root,
                renderer,
                get_provider,
            )
            .await?;

            if let review_loop::ReviewOutcome::Approved = outcome {
                if !options.dry_run {
                    run_planning_and_implementation(
                        config,
                        run_state,
                        run_dir,
                        runs_dir,
                        project_root,
                        renderer,
                        checkpoints,
                        get_provider,
                        false,
                        &mut metrics,
                    )
                    .await?;
                }
            }
        }
        Stage::Reviewing | Stage::SecurityAuditing | Stage::Judging => {
            let outcome = review_loop::run_review_loop(
                config,
                run_state,
                run_dir,
                runs_dir,
                project_root,
                renderer,
                get_provider,
            )
            .await?;

            if let review_loop::ReviewOutcome::Approved = outcome {
                if !options.dry_run {
                    run_planning_and_implementation(
                        config,
                        run_state,
                        run_dir,
                        runs_dir,
                        project_root,
                        renderer,
                        checkpoints,
                        get_provider,
                        false,
                        &mut metrics,
                    )
                    .await?;
                }
            }
        }
        Stage::Planning | Stage::TestArchitecting | Stage::Implementing => {
            run_planning_and_implementation(
                config,
                run_state,
                run_dir,
                runs_dir,
                project_root,
                renderer,
                checkpoints,
                get_provider,
                false,
                &mut metrics,
            )
            .await?;
        }
        Stage::Validating | Stage::Fixing => {
            renderer
                .info("resuming validation is not yet supported; restarting from implementation");
            run_planning_and_implementation(
                config,
                run_state,
                run_dir,
                runs_dir,
                project_root,
                renderer,
                checkpoints,
                get_provider,
                false,
                &mut metrics,
            )
            .await?;
        }
        Stage::AwaitingApproval(next_stage) => {
            let next_label = next_stage.label();
            if renderer.checkpoint_prompt(next_label) {
                run_state.advance(*next_stage);
                run_state.save(runs_dir)?;
                Box::pin(resume_pipeline(
                    config,
                    run_state,
                    run_dir,
                    runs_dir,
                    project_root,
                    renderer,
                    checkpoints,
                    get_provider,
                    options,
                ))
                .await?;
            } else {
                run_state.set_error("user declined at checkpoint during resume");
                renderer.info("run cancelled by user at checkpoint");
            }
        }
        Stage::Complete => {
            renderer.info("this run is already complete");
        }
        Stage::Failed(ref err) => {
            renderer.info(&format!("this run failed: {}", err));
            renderer.info("to retry, start a new run with the same request");
        }
    }

    Ok(())
}

pub fn effective_checkpoints(config: &Config, options: &PipelineOptions) -> Vec<Checkpoint> {
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

fn load_custom_instructions(
    project_root: &Path,
    agent_config: &crate::config::AgentConfig,
) -> Option<String> {
    let path = agent_config.custom_instructions.as_ref()?;
    let full_path = project_root.join(path);
    std::fs::read_to_string(full_path).ok()
}
