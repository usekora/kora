use anyhow::Result;
use clap::Parser;
use std::env;

use kora::cli::app::{Cli, Commands};
use kora::cli::configure;
use kora::config;
use kora::provider::detect_providers;
use kora::terminal::Renderer;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let project_root = env::current_dir()?;

    match cli.command {
        Some(Commands::Configure) => {
            configure::run_configure(&project_root)?;
        }
        Some(Commands::Run { request, .. }) => {
            println!("  run not yet implemented: {}", request);
        }
        Some(Commands::Resume) => {
            println!("  resume not yet implemented");
        }
        Some(Commands::History) => {
            println!("  history not yet implemented");
        }
        Some(Commands::Clean) => {
            println!("  clean not yet implemented");
        }
        None => {
            let config = config::load(&project_root)?;
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

            let input = kora::terminal::input::read_user_input()?;
            if input.is_empty() {
                return Ok(());
            }

            println!("\n  received: \"{}\"", input);
            println!("  pipeline not yet implemented — coming in Phase 2\n");
        }
    }

    Ok(())
}
