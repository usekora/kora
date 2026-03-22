use anyhow::Result;
use std::path::Path;

use crate::config::Config;
use crate::pipeline::{context, researcher, review_loop};
use crate::provider::{self, Provider};
use crate::state::{checkpoint_for_stage, Checkpoint, RunDirectory, RunState, Stage};
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
        load_custom_instructions(project_root, &config.agents.researcher)
            .as_deref(),
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
                renderer.info("dry run mode — stopping after review loop");
                run_state.advance(Stage::Complete);
                run_state.save(&runs_dir)?;
                return Ok(());
            }

            renderer.info("planning + implementation pipeline coming in Phase 3");
            run_state.advance(Stage::Complete);
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
