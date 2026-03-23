use anyhow::Result;
use clap::Parser;
use std::env;

use kora::cli::app::{Cli, Commands};
use kora::cli::configure;
use kora::cli::history;
use kora::cli::meta_commands::{self, MetaCommand};
use kora::cli::resume;
use kora::config;
use kora::config::presets;
use kora::pipeline::orchestrator::{self, PipelineOptions};
use kora::provider::{detect_providers, ProviderKind};
use kora::shutdown::{self, ShutdownSignal};
use kora::state::RunState;
use kora::terminal::Renderer;

fn main() -> Result<()> {
    let signal = ShutdownSignal::new();
    shutdown::install_ctrlc_handler(&signal);

    let result = run(signal);

    shutdown::restore_terminal();

    result
}

fn run(signal: ShutdownSignal) -> Result<()> {
    let cli = Cli::parse();
    let project_root = env::current_dir()?;

    match cli.command {
        Some(Commands::Configure) => {
            configure::run_configure(&project_root)?;
        }
        Some(Commands::Run {
            request,
            provider,
            profile,
            yolo,
            careful,
            dry_run,
        }) => {
            let mut config = config::load(&project_root)?;
            let detected = detect_providers();
            let mut renderer = Renderer::new();

            if detected.is_empty() {
                print_no_provider_error();
                return Ok(());
            }

            if !config::has_user_config(&project_root) {
                // Apply Balanced preset with detected providers on first run
                presets::apply_preset(config.pipeline_preset, &mut config.agents, &detected);
                renderer.info(&format!(
                    "using default configuration (preset: {}, {} checkpoints). run `kora configure` to customize.",
                    config.pipeline_preset,
                    config.checkpoints.len()
                ));
                renderer.text("");
            }

            let profile_override = profile.map(|p| {
                p.parse::<kora::state::PipelineProfile>()
                    .unwrap_or_else(|e| {
                        eprintln!("  warning: {} — defaulting to standard", e);
                        kora::state::PipelineProfile::Standard
                    })
            });

            let options = PipelineOptions {
                request,
                yolo,
                careful,
                dry_run,
                provider_override: provider,
                resume_run_id: None,
                profile_override,
            };

            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(async {
                tokio::select! {
                    result = orchestrator::run_pipeline(
                        &config,
                        &project_root,
                        options,
                        &mut renderer,
                    ) => result,
                    _ = signal.wait() => {
                        renderer.info("\n  pipeline interrupted — state saved, resumable with `kora resume`");
                        Ok(())
                    }
                }
            })?;
        }
        Some(Commands::Resume) => {
            resume::run_resume(&project_root)?;
        }
        Some(Commands::History) => {
            history::run_history(&project_root)?;
        }
        Some(Commands::Clean) => {
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(kora::cli::clean::run_clean(&project_root))?;
        }
        None => {
            run_interactive_session(&project_root, signal)?;
        }
    }

    Ok(())
}

fn run_interactive_session(project_root: &std::path::Path, signal: ShutdownSignal) -> Result<()> {
    let mut config = config::load(project_root)?;
    let detected = detect_providers();
    let mut renderer = Renderer::new();

    // Apply preset on first run if no user config exists
    if !detected.is_empty() && !config::has_user_config(project_root) {
        presets::apply_preset(config.pipeline_preset, &mut config.agents, &detected);
    }

    renderer.welcome(
        env!("CARGO_PKG_VERSION"),
        &config.default_provider,
        project_root,
    );

    if detected.is_empty() {
        renderer.no_providers_screen();
        return Ok(());
    }

    let mut last_run: Option<RunState> = None;

    loop {
        if signal.is_triggered() {
            break;
        }

        let branch = kora::terminal::input::get_git_branch(project_root);
        let status = kora::terminal::input::PromptStatus {
            preset: config.pipeline_preset.to_string(),
            branch,
            checkpoints: config.checkpoints.len(),
        };
        let input = kora::terminal::input::read_user_input(&status)?;
        if input.is_empty() {
            break;
        }

        match meta_commands::parse_meta_command(&input) {
            MetaCommand::Status => {
                kora::terminal::input::clear_last_input()?;
                meta_commands::handle_status(&mut renderer, last_run.as_ref());
                continue;
            }
            MetaCommand::Configure => {
                kora::terminal::input::clear_last_input()?;
                if let Err(e) = configure::run_configure(project_root) {
                    renderer.command_result(&format!("configure error: {}", e));
                } else {
                    config = config::load(project_root)?;
                }
                // Redraw screen — sub-menus may have scrolled past the welcome
                redraw_screen(&mut renderer, &config, project_root)?;
                continue;
            }
            MetaCommand::Clear => {
                kora::terminal::input::clear_last_input()?;
                last_run = None;
                redraw_screen(&mut renderer, &config, project_root)?;
                continue;
            }
            MetaCommand::Help => {
                kora::terminal::input::clear_last_input()?;
                meta_commands::handle_help(&mut renderer);
                continue;
            }
            MetaCommand::Quit => {
                break;
            }
            MetaCommand::None(request) => {
                renderer.echo_input(&input);
                let options = PipelineOptions {
                    request: request.clone(),
                    yolo: false,
                    careful: false,
                    dry_run: false,
                    provider_override: None,
                    resume_run_id: None,
                    profile_override: None,
                };

                let pipeline_signal = signal.clone_signal();
                let rt = tokio::runtime::Runtime::new()?;
                let result = rt.block_on(async {
                    tokio::select! {
                        result = orchestrator::run_pipeline(
                            &config,
                            project_root,
                            options,
                            &mut renderer,
                        ) => result,
                        _ = pipeline_signal.wait() => {
                            renderer.info("\r\n  pipeline interrupted — state saved, resumable with `kora resume`");
                            Ok(())
                        }
                    }
                });

                if let Err(e) = result {
                    renderer.info(&format!("  error: {}", e));
                }

                if signal.is_triggered() {
                    break;
                }

                let runs_dir = config::runs_dir();
                if let Ok(runs) = load_latest_run(&runs_dir) {
                    last_run = Some(runs);
                }

                renderer.interaction_break();
            }
        }
    }

    Ok(())
}

fn redraw_screen(
    renderer: &mut Renderer,
    config: &config::Config,
    project_root: &std::path::Path,
) -> Result<()> {
    crossterm::execute!(
        std::io::stdout(),
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
        crossterm::cursor::MoveTo(0, 0),
    )?;
    renderer.welcome(
        env!("CARGO_PKG_VERSION"),
        &config.default_provider,
        project_root,
    );
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

fn print_no_provider_error() {
    eprintln!();
    eprintln!("  No AI CLI tools detected.");
    eprintln!();
    eprintln!("  Kora needs at least one AI coding agent installed:");
    eprintln!();

    let providers = [
        (
            ProviderKind::Claude,
            "https://docs.anthropic.com/en/docs/claude-code",
        ),
        (ProviderKind::Codex, "https://github.com/openai/codex"),
        (
            ProviderKind::Gemini,
            "https://github.com/google-gemini/gemini-cli",
        ),
    ];

    for (kind, url) in providers {
        let installed = which::which(kind.cli_name()).is_ok();
        let status = if installed {
            "✓ installed"
        } else {
            "✗ not found"
        };
        eprintln!("    {}  {}", status, kind.cli_name());
        if !installed {
            eprintln!("         {}", url);
        }
    }

    eprintln!();
    eprintln!("  Install a provider, then run `kora configure` to set up.");
    eprintln!();
}
