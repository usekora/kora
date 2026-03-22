use anyhow::Result;
use clap::Parser;
use std::env;

use kora::cli::app::{Cli, Commands};
use kora::cli::configure;
use kora::cli::history;
use kora::cli::meta_commands::{self, MetaCommand};
use kora::cli::resume;
use kora::config;
use kora::pipeline::orchestrator::{self, PipelineOptions};
use kora::provider::detect_providers;
use kora::state::RunState;
use kora::terminal::verbosity::VerbosityState;
use kora::terminal::Renderer;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = env::current_dir()?;

    match cli.command {
        Some(Commands::Configure) => {
            configure::run_configure(&project_root)?;
        }
        Some(Commands::Run {
            request,
            provider,
            yolo,
            careful,
            dry_run,
        }) => {
            let config = config::load(&project_root)?;
            let detected = detect_providers();
            let mut renderer = Renderer::new();

            if detected.is_empty() {
                eprintln!("  No AI CLI tools detected. Install claude or codex first.");
                return Ok(());
            }

            let options = PipelineOptions {
                request,
                yolo,
                careful,
                dry_run,
                provider_override: provider,
                resume_run_id: None,
            };

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(orchestrator::run_pipeline(
                &config,
                &project_root,
                options,
                &mut renderer,
            ))?;
        }
        Some(Commands::Resume) => {
            resume::run_resume(&project_root)?;
        }
        Some(Commands::History) => {
            history::run_history(&project_root)?;
        }
        Some(Commands::Clean) => {
            let config = config::load(&project_root)?;
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(kora::cli::clean::run_clean(&project_root))?;
            let _ = config;
        }
        None => {
            run_interactive_session(&project_root)?;
        }
    }

    Ok(())
}

fn run_interactive_session(project_root: &std::path::Path) -> Result<()> {
    let config = config::load(project_root)?;
    let detected = detect_providers();
    let mut renderer = Renderer::new();

    if detected.is_empty() {
        eprintln!("  No AI CLI tools detected. Install claude or codex first.");
        eprintln!("  Run `kora configure` after installing a provider.");
        return Ok(());
    }

    renderer.welcome(
        env!("CARGO_PKG_VERSION"),
        &config.default_provider,
        config.checkpoints.len(),
    );

    let mut verbosity = VerbosityState::new(config.output.default_verbosity);
    let mut last_run: Option<RunState> = None;

    loop {
        let input = kora::terminal::input::read_user_input()?;
        if input.is_empty() {
            break;
        }

        match meta_commands::parse_meta_command(&input) {
            MetaCommand::Status => {
                meta_commands::handle_status(&mut renderer, last_run.as_ref());
                continue;
            }
            MetaCommand::Config => {
                meta_commands::handle_config(&mut renderer, &config);
                continue;
            }
            MetaCommand::Verbose => {
                meta_commands::handle_verbose(&mut renderer, &mut verbosity);
                continue;
            }
            MetaCommand::Help => {
                meta_commands::handle_help(&mut renderer);
                continue;
            }
            MetaCommand::Quit => {
                break;
            }
            MetaCommand::None(request) => {
                let options = PipelineOptions {
                    request: request.clone(),
                    yolo: false,
                    careful: false,
                    dry_run: false,
                    provider_override: None,
                    resume_run_id: None,
                };

                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(orchestrator::run_pipeline(
                    &config,
                    project_root,
                    options,
                    &mut renderer,
                ))?;

                let runs_dir = project_root.join(&config.runs_dir);
                if let Ok(runs) = load_latest_run(&runs_dir) {
                    last_run = Some(runs);
                }

                renderer.info("");
                renderer.info("ready for next request. type /help for commands, ctrl+c to exit.");
                renderer.text("");
            }
        }
    }

    Ok(())
}

fn load_latest_run(runs_dir: &std::path::Path) -> Result<RunState> {
    if !runs_dir.exists() {
        anyhow::bail!("no runs directory");
    }

    let mut latest: Option<RunState> = None;

    for entry in std::fs::read_dir(runs_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let run_id = entry.file_name().to_string_lossy().to_string();
            if let Ok(state) = RunState::load(runs_dir, &run_id) {
                if latest
                    .as_ref()
                    .map(|l| state.updated_at > l.updated_at)
                    .unwrap_or(true)
                {
                    latest = Some(state);
                }
            }
        }
    }

    latest.ok_or_else(|| anyhow::anyhow!("no runs found"))
}
