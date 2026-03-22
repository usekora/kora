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
    Gemini,
}

impl ProviderKind {
    pub fn cli_name(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "claude",
            ProviderKind::Codex => "codex",
            ProviderKind::Gemini => "gemini",
        }
    }

    pub fn autonomous_flags(&self) -> Vec<&'static str> {
        match self {
            ProviderKind::Claude => vec!["--dangerously-skip-permissions"],
            ProviderKind::Codex => vec!["--approval-mode", "full-auto"],
            ProviderKind::Gemini => vec!["--sandbox"],
        }
    }

    pub fn available_models(&self) -> Vec<&'static str> {
        match self {
            ProviderKind::Claude => vec![
                "opus-4-6-1m",
                "opus-4-6",
                "sonnet-4-6",
                "haiku-4-5",
            ],
            ProviderKind::Codex => vec![
                "gpt-5.4",
                "gpt-5.3-codex-spark",
                "gpt-5.3-codex",
                "gpt-5.2-codex",
            ],
            ProviderKind::Gemini => vec![
                "gemini-3.1-pro",
                "gemini-3-flash",
                "gemini-3.1-flash-lite",
            ],
        }
    }

    pub fn model_flag(&self) -> &'static str {
        match self {
            ProviderKind::Claude => "--model",
            ProviderKind::Codex => "--model",
            ProviderKind::Gemini => "--model",
        }
    }

    pub fn non_interactive_flags(&self) -> Vec<&'static str> {
        match self {
            ProviderKind::Claude => vec!["--print"],
            ProviderKind::Codex => vec!["--quiet"],
            ProviderKind::Gemini => vec![],
        }
    }
}
