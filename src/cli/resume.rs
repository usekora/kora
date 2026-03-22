use anyhow::Result;
use std::path::Path;

use crate::pipeline::orchestrator::{self, PipelineOptions};
use crate::provider::detect_providers;
use crate::state::{RunDirectory, RunState};
use crate::terminal::selector;
use crate::terminal::Renderer;

pub fn run_resume(project_root: &Path) -> Result<()> {
    let config = crate::config::load(project_root)?;
    let detected = detect_providers();

    if detected.is_empty() {
        eprintln!("  No AI CLI tools detected. Install claude or codex first.");
        return Ok(());
    }

    let runs_dir = crate::config::runs_dir();
    let interrupted = RunDirectory::list_interrupted(&runs_dir)?;

    if interrupted.is_empty() {
        println!("  no interrupted runs found");
        return Ok(());
    }

    let selected_run = if interrupted.len() == 1 {
        let run = &interrupted[0];
        let display = format_run_summary(run);
        println!("\n  {}", display);
        println!();
        run.clone()
    } else {
        let options: Vec<String> = interrupted.iter().map(format_run_summary).collect();
        let option_refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
        let idx = selector::select("Select a run to resume:", &option_refs, 0)?;
        interrupted[idx].clone()
    };

    let mut renderer = Renderer::new();
    renderer.info(&format!(
        "resuming run {} from stage: {}",
        selected_run.id,
        selected_run.status.label()
    ));

    let options = PipelineOptions {
        request: selected_run.request.clone(),
        yolo: false,
        careful: false,
        dry_run: false,
        provider_override: None,
        resume_run_id: Some(selected_run.id.clone()),
        profile_override: selected_run.pipeline_profile,
    };

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(orchestrator::run_pipeline(
        &config,
        project_root,
        options,
        &mut renderer,
    ))?;

    Ok(())
}

fn format_run_summary(run: &RunState) -> String {
    let request_display = if run.request.len() > 50 {
        format!("{}...", &run.request[..50])
    } else {
        run.request.clone()
    };

    let age = chrono::Utc::now()
        .signed_duration_since(run.updated_at)
        .num_minutes();
    let age_display = if age < 60 {
        format!("{} min ago", age)
    } else if age < 1440 {
        format!("{} hours ago", age / 60)
    } else {
        format!("{} days ago", age / 1440)
    };

    format!(
        "\"{}\" · {} · {}",
        request_display,
        run.status.label(),
        age_display
    )
}
