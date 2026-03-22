mod traits;
mod claude;
mod codex;
mod detection;

pub use traits::{AgentOutput, InteractiveSession, Provider, ProviderKind};
pub use claude::ClaudeProvider;
pub use codex::CodexProvider;
pub use detection::{detect_providers, DetectedProvider};

use crate::config::Config;

pub fn create_provider(config: &Config, agent_provider: &str) -> Option<Box<dyn Provider>> {
    let provider_name: &str = if agent_provider == "default" {
        &config.default_provider
    } else {
        agent_provider
    };

    let provider_config = config.providers.get(provider_name)?;

    match provider_name {
        "claude" => Some(Box::new(ClaudeProvider::new(&provider_config.cli))),
        "codex" => Some(Box::new(CodexProvider::new(&provider_config.cli))),
        _ => None,
    }
}
