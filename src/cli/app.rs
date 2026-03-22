use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "kora",
    version,
    about = "Multi-agent development orchestration CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Start a one-shot run with a specific request
    Run {
        /// The request to execute
        request: String,

        /// Override default provider
        #[arg(short, long)]
        provider: Option<String>,

        /// No checkpoints, full autopilot
        #[arg(long)]
        yolo: bool,

        /// Checkpoints at every stage
        #[arg(long)]
        careful: bool,

        /// Research + review only, no implementation
        #[arg(long)]
        dry_run: bool,
    },

    /// Interactive setup wizard
    Configure,

    /// Resume an interrupted session
    Resume,

    /// View past runs
    History,

    /// Clean up old run data
    Clean,
}
