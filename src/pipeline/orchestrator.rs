use anyhow::{Context as AnyhowContext, Result};
use std::path::Path;

use crate::config::Config;
use crate::pipeline::{context, implementation, planner, researcher, review_loop, test_architect};
use crate::provider::{self, Provider};
use crate::state::{checkpoint_for_stage, Checkpoint, RunDirectory, RunState, Stage};
use crate::terminal::dashboard::Dashboard;
use crate::terminal::Renderer;

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
    let mut run_state = RunState::new(&options.request);
    let runs_dir = project_root.join(&config.runs_dir);
    let run_dir = RunDirectory::new(&runs_dir, &run_state.id);
    run_dir.create_structure()?;
    run_state.save(&runs_dir)?;

    let checkpoints = effective_checkpoints(config, &options);

    let get_provider = |agent_provider: &str| -> Option<Box<dyn Provider>> {
        if let Some(ref override_name) = options.provider_override {
            provider::create_provider(config, override_name)
        } else {
            provider::create_provider(config, agent_provider)
        }
    };

    renderer.stage_header("researcher", "starting");

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

    renderer.stage_complete("researcher", 0);

    if should_checkpoint(&Stage::Reviewing, &checkpoints)
        && !renderer.checkpoint_prompt("review loop")
    {
        run_state.set_error("user declined at researcher checkpoint");
        run_state.save(&runs_dir)?;
        renderer.info("run cancelled by user at researcher checkpoint");
        return Ok(());
    }

    let outcome = review_loop::run_review_loop(
        config,
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

            if should_checkpoint(&Stage::Planning, &checkpoints)
                && !renderer.checkpoint_prompt("planning")
            {
                run_state.set_error("user declined at review loop checkpoint");
                run_state.save(&runs_dir)?;
                renderer.info("run cancelled by user at review loop checkpoint");
                return Ok(());
            }

            if options.dry_run {
                renderer.info("dry run mode -- stopping after review loop");
                run_state.advance(Stage::Complete);
                run_state.save(&runs_dir)?;
                return Ok(());
            }

            run_planning_and_implementation(
                config,
                &mut run_state,
                &run_dir,
                &runs_dir,
                project_root,
                renderer,
                &checkpoints,
                &get_provider,
            )
            .await?;
        }
        review_loop::ReviewOutcome::Escalated { iteration, reason } => {
            renderer.escalation(&format!(
                "review loop escalated after {} iterations: {}",
                iteration, reason
            ));
            run_state.set_error(&reason);
        }
    }

    run_state.save(&runs_dir)?;
    Ok(())
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
) -> Result<()> {
    let no_flags: Vec<String> = vec![];

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
        run_dir,
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
        run_dir,
        &no_flags,
    )
    .await?;

    renderer.stage_complete("test architect", 0);

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

    if failed.is_empty() {
        renderer.implementation_complete(fleet.total_tasks(), 0);
        run_state.advance(Stage::Complete);
    } else {
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
