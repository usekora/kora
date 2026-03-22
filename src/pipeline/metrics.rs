use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Estimate token count from text. Rough approximation: ~4 characters per token.
pub fn estimate_tokens(text: &str) -> u64 {
    (text.len() as u64).div_ceil(4)
}

/// Format a number with comma separators for thousands.
fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

/// Format a duration as human-readable string.
fn format_duration(d: Duration) -> String {
    let total_secs = d.as_secs();
    if total_secs >= 60 {
        let mins = total_secs / 60;
        let secs = total_secs % 60;
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", total_secs)
    }
}

/// A single agent invocation record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInvocation {
    pub agent_name: String,
    pub provider: String,
    pub duration_secs: u64,
    pub estimated_input_tokens: u64,
    pub estimated_output_tokens: u64,
}

/// Accumulated metrics for an entire pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetrics {
    pub invocations: Vec<AgentInvocation>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl RunMetrics {
    /// Create a new empty RunMetrics with started_at set to now.
    pub fn new() -> Self {
        Self {
            invocations: Vec::new(),
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    /// Record a single agent invocation, estimating tokens from input/output text.
    pub fn record(
        &mut self,
        agent_name: &str,
        provider: &str,
        duration: Duration,
        input_text: &str,
        output_text: &str,
    ) {
        let invocation = AgentInvocation {
            agent_name: agent_name.to_string(),
            provider: provider.to_string(),
            duration_secs: duration.as_secs(),
            estimated_input_tokens: estimate_tokens(input_text),
            estimated_output_tokens: estimate_tokens(output_text),
        };
        self.invocations.push(invocation);
    }

    /// Mark the run as completed with current timestamp.
    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
    }

    /// Sum of all invocation durations.
    pub fn total_duration(&self) -> Duration {
        let total_secs: u64 = self.invocations.iter().map(|i| i.duration_secs).sum();
        Duration::from_secs(total_secs)
    }

    /// Sum of input + output tokens across all invocations.
    pub fn total_estimated_tokens(&self) -> u64 {
        self.invocations
            .iter()
            .map(|i| i.estimated_input_tokens + i.estimated_output_tokens)
            .sum()
    }

    /// Number of invocations.
    pub fn agent_count(&self) -> usize {
        self.invocations.len()
    }

    /// Save metrics to `run_dir/metrics.json` as pretty JSON.
    pub fn save(&self, run_dir: &Path) -> anyhow::Result<()> {
        let path = run_dir.join("metrics.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load metrics from `run_dir/metrics.json`.
    pub fn load(run_dir: &Path) -> anyhow::Result<Self> {
        let path = run_dir.join("metrics.json");
        let json = std::fs::read_to_string(path)?;
        let metrics: Self = serde_json::from_str(&json)?;
        Ok(metrics)
    }

    /// Return formatted summary lines for display.
    pub fn summary_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // Invocation count
        lines.push(format!("agents: {} invocations", self.agent_count()));

        // Total time
        lines.push(format!(
            "total time: {}",
            format_duration(self.total_duration())
        ));

        // Token summary
        let total_input: u64 = self
            .invocations
            .iter()
            .map(|i| i.estimated_input_tokens)
            .sum();
        let total_output: u64 = self
            .invocations
            .iter()
            .map(|i| i.estimated_output_tokens)
            .sum();
        let total_tokens = total_input + total_output;
        lines.push(format!(
            "estimated tokens: ~{} (input: ~{} + output: ~{})",
            format_with_commas(total_tokens),
            format_with_commas(total_input),
            format_with_commas(total_output),
        ));

        // Per-agent breakdown
        for inv in &self.invocations {
            let tokens = inv.estimated_input_tokens + inv.estimated_output_tokens;
            lines.push(format!(
                "  {} ({}): {}s · ~{} tokens",
                inv.agent_name,
                inv.provider,
                inv.duration_secs,
                format_with_commas(tokens),
            ));
        }

        lines
    }
}

impl Default for RunMetrics {
    fn default() -> Self {
        Self::new()
    }
}
