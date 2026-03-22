use anyhow::Result;
use std::path::Path;
use std::time::Duration;

pub struct AgentOutput {
    pub text: String,
    pub exit_code: i32,
    pub duration: Duration,
}

pub struct InteractiveSession {
    pub child: tokio::process::Child,
}

#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;
    fn kind(&self) -> ProviderKind;
    fn is_available(&self) -> Result<bool>;

    async fn run(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
        timeout: Option<Duration>,
    ) -> Result<AgentOutput>;

    async fn run_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
        extra_flags: &[String],
    ) -> Result<InteractiveSession>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderKind {
    Claude,
    Codex,
}

impl ProviderKind {
    pub fn cli_name(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "claude",
            ProviderKind::Codex => "codex",
        }
    }

    pub fn autonomous_flags(&self) -> Vec<&'static str> {
        match self {
            ProviderKind::Claude => vec!["--dangerously-skip-permissions"],
            ProviderKind::Codex => vec!["--approval-mode", "full-auto"],
        }
    }

    pub fn non_interactive_flags(&self) -> Vec<&'static str> {
        match self {
            ProviderKind::Claude => vec!["--print"],
            ProviderKind::Codex => vec!["--quiet"],
        }
    }
}
