use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use crate::state::Checkpoint;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PipelinePreset {
    Quality,
    #[default]
    Balanced,
    Speed,
    Custom,
}

impl fmt::Display for PipelinePreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelinePreset::Quality => write!(f, "Quality"),
            PipelinePreset::Balanced => write!(f, "Balanced"),
            PipelinePreset::Speed => write!(f, "Speed"),
            PipelinePreset::Custom => write!(f, "Custom"),
        }
    }
}

impl PipelinePreset {
    /// Cycle to the next preset: Quality -> Balanced -> Speed -> Custom -> Quality
    pub fn next(self) -> Self {
        match self {
            PipelinePreset::Quality => PipelinePreset::Balanced,
            PipelinePreset::Balanced => PipelinePreset::Speed,
            PipelinePreset::Speed => PipelinePreset::Custom,
            PipelinePreset::Custom => PipelinePreset::Quality,
        }
    }

    /// Quality bar for display (1-5 filled squares)
    pub fn quality_bar(self) -> &'static str {
        match self {
            PipelinePreset::Quality => "\u{25a0}\u{25a0}\u{25a0}\u{25a0}\u{25a0}",
            PipelinePreset::Balanced => "\u{25a0}\u{25a0}\u{25a0}\u{25a1}\u{25a1}",
            PipelinePreset::Speed => "\u{25a0}\u{25a0}\u{25a1}\u{25a1}\u{25a1}",
            PipelinePreset::Custom => "",
        }
    }

    /// Speed bar for display (1-5 filled squares)
    pub fn speed_bar(self) -> &'static str {
        match self {
            PipelinePreset::Quality => "\u{25a0}\u{25a0}\u{25a1}\u{25a1}\u{25a1}",
            PipelinePreset::Balanced => "\u{25a0}\u{25a0}\u{25a0}\u{25a1}\u{25a1}",
            PipelinePreset::Speed => "\u{25a0}\u{25a0}\u{25a0}\u{25a0}\u{25a0}",
            PipelinePreset::Custom => "",
        }
    }

    pub fn description(self, has_claude: bool, has_codex: bool, has_gemini: bool) -> String {
        match self {
            PipelinePreset::Quality => {
                let mut parts = Vec::new();
                if has_claude {
                    parts.push("Claude for planning & implementation");
                }
                if has_gemini {
                    parts.push("Gemini for research");
                }
                if has_codex {
                    parts.push("Codex for test & validation");
                }
                if parts.is_empty() {
                    return "Best provider for each role".to_string();
                }
                parts.join(", ")
            }
            PipelinePreset::Balanced => {
                let mut parts = Vec::new();
                if has_claude {
                    parts.push("Claude for core pipeline");
                }
                if has_gemini {
                    parts.push("Gemini for research");
                }
                if has_codex {
                    parts.push("Codex for lightweight tasks");
                }
                if parts.is_empty() {
                    return "Good balance of quality and speed".to_string();
                }
                parts.join(", ")
            }
            PipelinePreset::Speed => {
                "Fastest provider per role, skips security & test agents".to_string()
            }
            PipelinePreset::Custom => "Per-agent provider and model control".to_string(),
        }
    }

    pub fn all() -> &'static [PipelinePreset] {
        &[
            PipelinePreset::Quality,
            PipelinePreset::Balanced,
            PipelinePreset::Speed,
            PipelinePreset::Custom,
        ]
    }
}

fn default_preset() -> PipelinePreset {
    PipelinePreset::Balanced
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub version: u32,
    pub default_provider: String,
    #[serde(default = "default_preset")]
    pub pipeline_preset: PipelinePreset,
    pub providers: HashMap<String, ProviderConfig>,
    pub agents: AgentsConfig,
    pub checkpoints: Vec<Checkpoint>,
    pub review_loop: LoopConfig,
    pub validation_loop: LoopConfig,
    pub implementation: ImplementationConfig,
    pub output: OutputConfig,
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
    #[serde(default)]
    pub model: Option<String>,
    pub custom_instructions: Option<PathBuf>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl AgentConfig {
    /// Build extra CLI flags for this agent (e.g., --model).
    pub fn extra_flags(&self) -> Vec<String> {
        let mut flags = Vec::new();
        if let Some(model) = &self.model {
            flags.push("--model".to_string());
            flags.push(model.clone());
        }
        flags
    }
}

fn default_provider() -> String {
    "default".to_string()
}

fn default_timeout() -> u64 {
    300
}

fn default_enabled() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentsConfig {
    pub researcher: AgentConfig,
    pub plan_reviewer: AgentConfig,
    pub plan_security_auditor: AgentConfig,
    pub judge: AgentConfig,
    pub planner: AgentConfig,
    pub test_architect: AgentConfig,
    pub implementor: AgentConfig,
    pub code_reviewer: AgentConfig,
    pub code_security_auditor: AgentConfig,
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
            model: None,
            custom_instructions: None,
            timeout_seconds: 300,
            enabled: true,
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
        providers.insert(
            "gemini".to_string(),
            ProviderConfig {
                cli: "gemini".to_string(),
                flags: vec![],
            },
        );

        Self {
            version: 1,
            default_provider: "claude".to_string(),
            pipeline_preset: PipelinePreset::Balanced,
            providers,
            agents: AgentsConfig {
                researcher: default_agent.clone(),
                plan_reviewer: default_agent.clone(),
                plan_security_auditor: default_agent.clone(),
                judge: default_agent.clone(),
                planner: default_agent.clone(),
                test_architect: default_agent.clone(),
                implementor: default_agent.clone(),
                code_reviewer: default_agent.clone(),
                code_security_auditor: default_agent.clone(),
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
        }
    }
}
