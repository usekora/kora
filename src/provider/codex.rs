use anyhow::Result;
use std::path::Path;
use std::process::Stdio;
use std::time::Instant;
use tokio::process::Command;

use super::traits::{AgentOutput, InteractiveSession, Provider, ProviderKind};

pub struct CodexProvider {
    cli_path: String,
}

impl CodexProvider {
    pub fn new(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
        }
    }

    fn base_command(&self, working_dir: &Path, extra_flags: &[String]) -> Command {
        let mut cmd = Command::new(&self.cli_path);
        cmd.current_dir(working_dir);
        for flag in ProviderKind::Codex.autonomous_flags() {
            cmd.arg(flag);
        }
        for flag in extra_flags {
            cmd.arg(flag);
        }
        cmd
    }
}

#[async_trait::async_trait]
impl Provider for CodexProvider {
    fn name(&self) -> &str {
        "codex"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Codex
    }

    fn is_available(&self) -> Result<bool> {
        Ok(which::which(&self.cli_path).is_ok())
    }

    async fn run(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<AgentOutput> {
        let start = Instant::now();
        let mut cmd = self.base_command(working_dir, extra_flags);
        for flag in ProviderKind::Codex.non_interactive_flags() {
            cmd.arg(flag);
        }
        cmd.arg(prompt);
        let output = cmd.output().await?;
        let duration = start.elapsed();

        Ok(AgentOutput {
            text: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration,
        })
    }

    async fn run_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<InteractiveSession> {
        let mut cmd = self.base_command(working_dir, extra_flags);
        cmd.arg(prompt);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let child = cmd.spawn()?;
        Ok(InteractiveSession { child })
    }
}
