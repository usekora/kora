use anyhow::{bail, Result};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

use super::traits::{AgentOutput, InteractiveSession, Provider, ProviderKind};

pub struct ClaudeProvider {
    cli_path: String,
}

impl ClaudeProvider {
    pub fn new(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
        }
    }

    fn base_command(&self, working_dir: &Path, extra_flags: &[String]) -> Command {
        let mut cmd = Command::new(&self.cli_path);
        cmd.current_dir(working_dir);
        for flag in ProviderKind::Claude.autonomous_flags() {
            cmd.arg(flag);
        }
        for flag in extra_flags {
            cmd.arg(flag);
        }
        cmd
    }
}

#[async_trait::async_trait]
impl Provider for ClaudeProvider {
    fn name(&self) -> &str {
        "claude"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Claude
    }

    fn is_available(&self) -> Result<bool> {
        Ok(which::which(&self.cli_path).is_ok())
    }

    async fn run(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
        timeout: Option<Duration>,
    ) -> Result<AgentOutput> {
        let start = Instant::now();
        let mut cmd = self.base_command(working_dir, extra_flags);
        for flag in ProviderKind::Claude.non_interactive_flags() {
            cmd.arg(flag);
        }
        cmd.arg("-p").arg(prompt);

        let output = match timeout {
            Some(duration) => {
                let child = cmd
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()?;
                match tokio::time::timeout(duration, child.wait_with_output()).await {
                    Ok(result) => result?,
                    Err(_) => {
                        bail!("claude agent timed out after {}s", duration.as_secs());
                    }
                }
            }
            None => cmd.output().await?,
        };

        Ok(AgentOutput {
            text: String::from_utf8_lossy(&output.stdout).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            duration: start.elapsed(),
        })
    }

    async fn run_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<InteractiveSession> {
        let mut cmd = self.base_command(working_dir, extra_flags);
        cmd.arg("-p").arg(prompt);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let child = cmd.spawn()?;
        Ok(InteractiveSession { child })
    }
}
