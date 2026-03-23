use anyhow::{bail, Result};
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;

use super::traits::{AgentOutput, InteractiveSession, Provider, ProviderKind};

pub struct GeminiProvider {
    cli_path: String,
}

impl GeminiProvider {
    pub fn new(cli_path: &str) -> Self {
        Self {
            cli_path: cli_path.to_string(),
        }
    }

    fn base_command(&self, working_dir: &Path, extra_flags: &[String]) -> Command {
        let mut cmd = Command::new(&self.cli_path);
        cmd.current_dir(working_dir);
        cmd.kill_on_drop(true);
        for flag in ProviderKind::Gemini.autonomous_flags() {
            cmd.arg(flag);
        }
        for flag in extra_flags {
            cmd.arg(flag);
        }
        cmd
    }
}

#[async_trait::async_trait]
impl Provider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn kind(&self) -> ProviderKind {
        ProviderKind::Gemini
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
        for flag in ProviderKind::Gemini.non_interactive_flags() {
            cmd.arg(flag);
        }

        let large_prompt = prompt.len() > 50_000;
        if !large_prompt {
            cmd.arg(prompt);
            cmd.stdin(Stdio::null());
        } else {
            cmd.stdin(Stdio::piped());
        }

        let output = if large_prompt {
            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = cmd.spawn()?;
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                stdin.write_all(prompt.as_bytes()).await?;
                drop(stdin);
            }
            match timeout {
                Some(duration) => {
                    match tokio::time::timeout(duration, child.wait_with_output()).await {
                        Ok(result) => result?,
                        Err(_) => {
                            bail!("gemini agent timed out after {}s", duration.as_secs());
                        }
                    }
                }
                None => child.wait_with_output().await?,
            }
        } else {
            match timeout {
                Some(duration) => {
                    let child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
                    match tokio::time::timeout(duration, child.wait_with_output()).await {
                        Ok(result) => result?,
                        Err(_) => {
                            bail!("gemini agent timed out after {}s", duration.as_secs());
                        }
                    }
                }
                None => cmd.output().await?,
            }
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
        cmd.arg(prompt);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        let child = cmd.spawn()?;
        Ok(InteractiveSession { child })
    }
}
