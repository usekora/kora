use anyhow::{Context, Result};
use std::path::Path;
use std::time::Duration;

use crate::agent::output_parser;
use crate::config::Config;
use crate::pipeline::{context, researcher};
use crate::provider::Provider;
use crate::state::{RunDirectory, RunState, Stage};
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
    runs_dir: &Path,
    project_root: &Path,
    renderer: &mut Renderer,
    get_provider: &dyn Fn(&str) -> Option<Box<dyn Provider>>,
) -> Result<ReviewOutcome> {
    let max = config.review_loop.max_iterations;
    let no_flags: Vec<String> = vec![];

    for iteration in 1..=max {
        run_state.increment_iteration();
        renderer.iteration_header(iteration, max);

        let iter_dir = run_dir.reviews_dir(iteration);
        std::fs::create_dir_all(&iter_dir)?;

        run_state.advance(Stage::Reviewing);
        run_state.save(runs_dir)?;

        let reviewer_provider = get_provider(&config.agents.reviewer.provider)
            .context("no provider available for reviewer")?;
        let security_provider = get_provider(&config.agents.security_auditor.provider)
            .context("no provider available for security auditor")?;

        let review_prompt = context::build_reviewer_prompt(
            &runs_dir.join(&run_state.id),
            iteration,
            &run_state.request,
            project_root,
            config.agents.reviewer.custom_instructions.as_deref(),
        )?;
        let security_prompt = context::build_security_prompt(
            &runs_dir.join(&run_state.id),
            iteration,
            &run_state.request,
            project_root,
            config
                .agents
                .security_auditor
                .custom_instructions
                .as_deref(),
        )?;

        let reviewer_timeout = Some(Duration::from_secs(config.agents.reviewer.timeout_seconds));
        let security_timeout = Some(Duration::from_secs(
            config.agents.security_auditor.timeout_seconds,
        ));

        let (review_result, security_result) = tokio::join!(
            reviewer_provider.run(
                &review_prompt.prompt,
                project_root,
                &no_flags,
                reviewer_timeout
            ),
            security_provider.run(
                &security_prompt.prompt,
                project_root,
                &no_flags,
                security_timeout
            ),
        );

        let review_output = review_result.context("reviewer failed")?;
        let security_output = security_result.context("security auditor failed")?;

        std::fs::write(iter_dir.join("review.md"), &review_output.text)?;
        std::fs::write(iter_dir.join("security-audit.md"), &security_output.text)?;

        renderer.stage_complete("reviewer", review_output.duration.as_secs());
        renderer.stage_complete("security auditor", security_output.duration.as_secs());

        if let Some(review_summary) = output_parser::parse_review(&review_output.text) {
            for f in &review_summary.findings {
                renderer.finding(&f.severity, &f.title);
            }
        }
        if let Some(security_summary) = output_parser::parse_security_review(&security_output.text)
        {
            for f in &security_summary.findings {
                renderer.finding(&f.severity, &f.title);
            }
        }

        run_state.advance(Stage::Judging);
        run_state.save(runs_dir)?;

        renderer.stage_header("judge", "evaluating");

        let judge_provider = get_provider(&config.agents.judge.provider)
            .context("no provider available for judge")?;
        let judge_prompt = context::build_judge_prompt(
            &runs_dir.join(&run_state.id),
            iteration,
            &run_state.request,
            project_root,
            config.agents.judge.custom_instructions.as_deref(),
        )?;

        let judge_timeout = Some(Duration::from_secs(config.agents.judge.timeout_seconds));
        let judge_output = judge_provider
            .run(&judge_prompt.prompt, project_root, &no_flags, judge_timeout)
            .await
            .context("judge failed")?;

        std::fs::write(iter_dir.join("judgment.md"), &judge_output.text)?;

        renderer.stage_complete("judge", judge_output.duration.as_secs());

        let verdict = output_parser::parse_verdict(&judge_output.text);

        match verdict {
            Some(v) => {
                renderer.review_loop_summary(
                    iteration,
                    v.valid_count,
                    v.dismissed_count,
                    &v.overall,
                );

                for fv in &v.findings {
                    let accepted = fv.verdict.eq_ignore_ascii_case("VALID");
                    renderer.verdict_line(&fv.id, accepted, &fv.verdict);
                }

                if v.overall == "APPROVE" {
                    return Ok(ReviewOutcome::Approved);
                }

                if iteration < max {
                    run_state.advance(Stage::Researching);
                    run_state.save(runs_dir)?;

                    renderer.stage_header("researcher", "revising");

                    let researcher_provider = get_provider(&config.agents.researcher.provider)
                        .context("no provider available for researcher")?;
                    let revision_prompt = context::build_researcher_revision_prompt(
                        &runs_dir.join(&run_state.id),
                        iteration,
                        project_root,
                        config.agents.researcher.custom_instructions.as_deref(),
                    )?;

                    researcher::run_revision(
                        researcher_provider.as_ref(),
                        &revision_prompt.prompt,
                        project_root,
                        run_dir,
                        &no_flags,
                        config.agents.researcher.timeout_seconds,
                    )
                    .await?;

                    renderer.stage_complete("researcher (revision)", 0);
                }
            }
            None => {
                renderer.escalation("judge did not produce a parseable verdict");
                return Ok(ReviewOutcome::Escalated {
                    iteration,
                    reason: "judge output missing structured verdict markers".to_string(),
                });
            }
        }
    }

    Ok(ReviewOutcome::Escalated {
        iteration: max,
        reason: format!("review loop did not converge after {} iterations", max),
    })
}
