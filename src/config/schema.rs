use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::state::Checkpoint;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub version: u32,
    pub default_provider: String,
    pub providers: HashMap<String, ProviderConfig>,
    pub agents: AgentsConfig,
    pub checkpoints: Vec<Checkpoint>,
    pub review_loop: LoopConfig,
    pub validation_loop: LoopConfig,
    pub implementation: ImplementationConfig,
    pub output: OutputConfig,
    pub runs_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProviderConfig {
    pub cli: String,
    #[serde(default)]
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub custom_instructions: Option<PathBuf>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_provider() -> String {
    "default".to_string()
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentsConfig {
    pub researcher: AgentConfig,
    pub reviewer: AgentConfig,
    pub security_auditor: AgentConfig,
    pub judge: AgentConfig,
    pub planner: AgentConfig,
    pub test_architect: AgentConfig,
    pub implementor: AgentConfig,
    pub validator: AgentConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopConfig {
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImplementationConfig {
    pub branch_strategy: BranchStrategy,
    pub parallel_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum BranchStrategy {
    Separate,
    Single,
    PlannerDecides,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputConfig {
    pub default_verbosity: Verbosity,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Verbosity {
    Focused,
    Detailed,
    Verbose,
}

impl Default for Config {
    fn default() -> Self {
        let default_agent = AgentConfig {
            provider: "default".to_string(),
            custom_instructions: None,
            timeout_seconds: 300,
        };

        let mut providers = HashMap::new();
        providers.insert(
            "claude".to_string(),
            ProviderConfig {
                cli: "claude".to_string(),
                flags: vec![],
            },
        );
        providers.insert(
            "codex".to_string(),
            ProviderConfig {
                cli: "codex".to_string(),
                flags: vec![],
            },
        );

        Self {
            version: 1,
            default_provider: "claude".to_string(),
            providers,
            agents: AgentsConfig {
                researcher: default_agent.clone(),
                reviewer: default_agent.clone(),
                security_auditor: default_agent.clone(),
                judge: default_agent.clone(),
                planner: default_agent.clone(),
                test_architect: default_agent.clone(),
                implementor: default_agent.clone(),
                validator: default_agent,
            },
            checkpoints: vec![Checkpoint::AfterResearcher, Checkpoint::AfterPlanner],
            review_loop: LoopConfig { max_iterations: 3 },
            validation_loop: LoopConfig { max_iterations: 2 },
            implementation: ImplementationConfig {
                branch_strategy: BranchStrategy::Separate,
                parallel_limit: 4,
            },
            output: OutputConfig {
                default_verbosity: Verbosity::Focused,
            },
            runs_dir: PathBuf::from(".kora/runs"),
        }
    }
}
