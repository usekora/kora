use super::schema::{AgentsConfig, PipelinePreset};
use crate::provider::DetectedProvider;
use crate::provider::ProviderKind;

/// Apply a preset to the agent configuration based on detected providers.
///
/// Presets control which PROVIDER handles each agent role.
/// Model selection is left to each CLI's default — this avoids version
/// mismatches when a user's CLI doesn't support a specific model.
/// Users who want specific models can use Custom mode.
pub fn apply_preset(
    preset: PipelinePreset,
    agents: &mut AgentsConfig,
    detected: &[DetectedProvider],
) {
    if preset == PipelinePreset::Custom {
        return;
    }

    let has_claude = detected.iter().any(|p| p.kind == ProviderKind::Claude);
    let has_codex = detected.iter().any(|p| p.kind == ProviderKind::Codex);
    let has_gemini = detected.iter().any(|p| p.kind == ProviderKind::Gemini);

    match (has_claude, has_codex, has_gemini) {
        (true, true, true) => apply_all_three(preset, agents),
        (true, true, false) => apply_claude_codex(preset, agents),
        (true, false, true) => apply_claude_gemini(preset, agents),
        (true, false, false) => apply_claude_only(preset, agents),
        (false, true, true) => apply_codex_gemini(preset, agents),
        (false, true, false) => apply_codex_only(preset, agents),
        (false, false, true) => apply_gemini_only(preset, agents),
        (false, false, false) => return,
    }

    // Clear any previously set models — let CLIs use their defaults
    clear_all_models(agents);

    if preset == PipelinePreset::Speed {
        agents.plan_security_auditor.enabled = false;
        agents.code_security_auditor.enabled = false;
        agents.test_architect.enabled = false;
    } else {
        set_all_enabled(agents, true);
    }
}

// ── Claude + Codex + Gemini ──────────────────────────────────────────

fn apply_all_three(preset: PipelinePreset, agents: &mut AgentsConfig) {
    match preset {
        PipelinePreset::Quality => {
            // Gemini for research (massive context), Claude for thinking + implementation
            // Codex for structured tasks (test architect, validation)
            set_provider(&mut agents.researcher, "gemini");
            set_provider(&mut agents.planner, "claude");
            set_provider(&mut agents.judge, "claude");
            set_provider(&mut agents.implementor, "claude");
            set_provider(&mut agents.plan_reviewer, "claude");
            set_provider(&mut agents.code_reviewer, "claude");
            set_provider(&mut agents.plan_security_auditor, "claude");
            set_provider(&mut agents.code_security_auditor, "claude");
            set_provider(&mut agents.test_architect, "codex");
            set_provider(&mut agents.validator, "codex");
        }
        PipelinePreset::Balanced => {
            // Gemini for research, Claude for core pipeline, Codex for lightweight
            set_provider(&mut agents.researcher, "gemini");
            set_provider(&mut agents.planner, "claude");
            set_provider(&mut agents.judge, "claude");
            set_provider(&mut agents.implementor, "claude");
            set_provider(&mut agents.plan_reviewer, "claude");
            set_provider(&mut agents.code_reviewer, "claude");
            set_provider(&mut agents.plan_security_auditor, "claude");
            set_provider(&mut agents.code_security_auditor, "claude");
            set_provider(&mut agents.test_architect, "codex");
            set_provider(&mut agents.validator, "codex");
        }
        PipelinePreset::Speed => {
            // Gemini for research, Claude for core, Codex for review/validation
            set_provider(&mut agents.researcher, "gemini");
            set_provider(&mut agents.planner, "claude");
            set_provider(&mut agents.judge, "claude");
            set_provider(&mut agents.implementor, "claude");
            set_provider(&mut agents.plan_reviewer, "codex");
            set_provider(&mut agents.code_reviewer, "codex");
            set_provider(&mut agents.plan_security_auditor, "claude");
            set_provider(&mut agents.code_security_auditor, "claude");
            set_provider(&mut agents.test_architect, "codex");
            set_provider(&mut agents.validator, "codex");
        }
        PipelinePreset::Custom => {}
    }
}

// ── Claude + Codex ───────────────────────────────────────────────────

fn apply_claude_codex(preset: PipelinePreset, agents: &mut AgentsConfig) {
    match preset {
        PipelinePreset::Quality => {
            // Claude for everything except structured tasks
            set_all_providers(agents, "claude");
            set_provider(&mut agents.test_architect, "codex");
            set_provider(&mut agents.validator, "codex");
        }
        PipelinePreset::Balanced => {
            // Claude for core, Codex for lightweight
            set_all_providers(agents, "claude");
            set_provider(&mut agents.test_architect, "codex");
            set_provider(&mut agents.validator, "codex");
        }
        PipelinePreset::Speed => {
            // Claude for implementation + planning, Codex for everything else
            set_all_providers(agents, "codex");
            set_provider(&mut agents.researcher, "claude");
            set_provider(&mut agents.planner, "claude");
            set_provider(&mut agents.implementor, "claude");
        }
        PipelinePreset::Custom => {}
    }
}

// ── Claude + Gemini ──────────────────────────────────────────────────

fn apply_claude_gemini(preset: PipelinePreset, agents: &mut AgentsConfig) {
    match preset {
        PipelinePreset::Quality => {
            set_all_providers(agents, "claude");
            set_provider(&mut agents.researcher, "gemini");
        }
        PipelinePreset::Balanced => {
            set_all_providers(agents, "claude");
            set_provider(&mut agents.researcher, "gemini");
            set_provider(&mut agents.validator, "gemini");
        }
        PipelinePreset::Speed => {
            set_all_providers(agents, "claude");
            set_provider(&mut agents.researcher, "gemini");
            set_provider(&mut agents.validator, "gemini");
        }
        PipelinePreset::Custom => {}
    }
}

// ── Claude only ──────────────────────────────────────────────────────

fn apply_claude_only(_preset: PipelinePreset, agents: &mut AgentsConfig) {
    set_all_providers(agents, "claude");
}

// ── Codex + Gemini ───────────────────────────────────────────────────

fn apply_codex_gemini(preset: PipelinePreset, agents: &mut AgentsConfig) {
    match preset {
        PipelinePreset::Quality => {
            // Gemini for thinking, Codex for implementation
            set_provider(&mut agents.researcher, "gemini");
            set_provider(&mut agents.planner, "gemini");
            set_provider(&mut agents.judge, "gemini");
            set_provider(&mut agents.plan_reviewer, "gemini");
            set_provider(&mut agents.plan_security_auditor, "gemini");
            set_provider(&mut agents.implementor, "codex");
            set_provider(&mut agents.code_reviewer, "codex");
            set_provider(&mut agents.code_security_auditor, "codex");
            set_provider(&mut agents.test_architect, "codex");
            set_provider(&mut agents.validator, "codex");
        }
        PipelinePreset::Balanced => {
            set_provider(&mut agents.researcher, "gemini");
            set_provider(&mut agents.planner, "gemini");
            set_provider(&mut agents.judge, "codex");
            set_provider(&mut agents.implementor, "codex");
            set_provider(&mut agents.plan_reviewer, "codex");
            set_provider(&mut agents.code_reviewer, "codex");
            set_provider(&mut agents.plan_security_auditor, "gemini");
            set_provider(&mut agents.code_security_auditor, "codex");
            set_provider(&mut agents.test_architect, "codex");
            set_provider(&mut agents.validator, "codex");
        }
        PipelinePreset::Speed => {
            set_all_providers(agents, "codex");
            set_provider(&mut agents.researcher, "gemini");
        }
        PipelinePreset::Custom => {}
    }
}

// ── Codex only ───────────────────────────────────────────────────────

fn apply_codex_only(_preset: PipelinePreset, agents: &mut AgentsConfig) {
    set_all_providers(agents, "codex");
}

// ── Gemini only ──────────────────────────────────────────────────────

fn apply_gemini_only(_preset: PipelinePreset, agents: &mut AgentsConfig) {
    set_all_providers(agents, "gemini");
}

// ── Helpers ──────────────────────────────────────────────────────────

fn set_provider(agent: &mut super::schema::AgentConfig, provider: &str) {
    agent.provider = provider.to_string();
}

fn set_all_providers(agents: &mut AgentsConfig, provider: &str) {
    set_provider(&mut agents.researcher, provider);
    set_provider(&mut agents.plan_reviewer, provider);
    set_provider(&mut agents.plan_security_auditor, provider);
    set_provider(&mut agents.judge, provider);
    set_provider(&mut agents.planner, provider);
    set_provider(&mut agents.test_architect, provider);
    set_provider(&mut agents.implementor, provider);
    set_provider(&mut agents.code_reviewer, provider);
    set_provider(&mut agents.code_security_auditor, provider);
    set_provider(&mut agents.validator, provider);
}

fn clear_all_models(agents: &mut AgentsConfig) {
    agents.researcher.model = None;
    agents.plan_reviewer.model = None;
    agents.plan_security_auditor.model = None;
    agents.judge.model = None;
    agents.planner.model = None;
    agents.test_architect.model = None;
    agents.implementor.model = None;
    agents.code_reviewer.model = None;
    agents.code_security_auditor.model = None;
    agents.validator.model = None;
}

fn set_all_enabled(agents: &mut AgentsConfig, enabled: bool) {
    agents.researcher.enabled = enabled;
    agents.plan_reviewer.enabled = enabled;
    agents.plan_security_auditor.enabled = enabled;
    agents.judge.enabled = enabled;
    agents.planner.enabled = enabled;
    agents.test_architect.enabled = enabled;
    agents.implementor.enabled = enabled;
    agents.code_reviewer.enabled = enabled;
    agents.code_security_auditor.enabled = enabled;
    agents.validator.enabled = enabled;
}
