use anyhow::Result;
use std::path::Path;

use crate::config::{self, presets, AgentConfig, AgentsConfig, BranchStrategy, PipelinePreset};
use crate::provider::detect_providers;
use crate::state::Checkpoint;
use crate::terminal::selector::{self, Setting, SettingsAction};

/// The 10 agent keys, their readable labels, and accessor functions.
const AGENT_ENTRIES: &[(&str, &str)] = &[
    ("researcher", "Researcher"),
    ("plan_reviewer", "Plan Reviewer"),
    ("plan_security_auditor", "Plan Security Auditor"),
    ("judge", "Judge"),
    ("planner", "Planner"),
    ("test_architect", "Test Architect"),
    ("implementor", "Implementor"),
    ("code_reviewer", "Code Reviewer"),
    ("code_security_auditor", "Code Security Auditor"),
    ("validator", "Validator"),
];

/// The 6 optional agents that can be disabled (indices into AGENT_ENTRIES).
const DISABLEABLE_AGENTS: &[usize] = &[1, 2, 5, 7, 8, 9];

pub fn run_configure(project_root: &Path) -> Result<()> {
    let mut config = config::load(project_root)?;
    let detected = detect_providers();

    if detected.is_empty() {
        eprintln!("  No AI CLI tools detected. Install claude, codex, or gemini first.");
        return Ok(());
    }

    // Draw header once — stays fixed while editing settings
    selector::draw_settings_header()?;

    loop {
        let settings = build_settings(&config);
        match selector::settings_menu(&settings)? {
            SettingsAction::Edit(idx) => {
                edit_setting(idx, &mut config, &detected)?;
            }
            SettingsAction::Exit => break,
        }
    }

    // Clean up the fixed header
    selector::clear_settings_header()?;

    config::save_home(&config)?;

    Ok(())
}

fn build_settings(config: &config::Config) -> Vec<Setting> {
    let preset_value = config.pipeline_preset.to_string();

    let agent_config_value = agent_models_summary(&config.agents);

    vec![
        Setting {
            label: "Pipeline preset".to_string(),
            value: preset_value,
        },
        Setting {
            label: "Agent config".to_string(),
            value: agent_config_value,
        },
        Setting {
            label: "Disabled agents".to_string(),
            value: disabled_agents_summary(&config.agents),
        },
        Setting {
            label: "Checkpoints".to_string(),
            value: if config.checkpoints.is_empty() {
                "none".to_string()
            } else {
                config
                    .checkpoints
                    .iter()
                    .map(checkpoint_label)
                    .collect::<Vec<_>>()
                    .join(", ")
            },
        },
        Setting {
            label: "Branch strategy".to_string(),
            value: branch_label(&config.implementation.branch_strategy).to_string(),
        },
        Setting {
            label: "Max parallel tasks".to_string(),
            value: config.implementation.parallel_limit.to_string(),
        },
    ]
}

fn edit_setting(
    idx: usize,
    config: &mut config::Config,
    detected: &[crate::provider::DetectedProvider],
) -> Result<()> {
    match idx {
        0 => select_preset(config, detected)?,
        1 => {
            edit_agent_models(config, detected)?;
            // If agent config was changed while on a preset, switch to Custom
            config.pipeline_preset = PipelinePreset::Custom;
        }
        2 => edit_disabled_agents(config)?,
        3 => edit_checkpoints(config)?,
        4 => edit_branch_strategy(config)?,
        5 => edit_parallel_limit(config)?,
        _ => {}
    }
    Ok(())
}

// --- Pipeline preset (panel) ---

fn select_preset(
    config: &mut config::Config,
    detected: &[crate::provider::DetectedProvider],
) -> Result<()> {
    use crate::provider::ProviderKind;
    let has_claude = detected.iter().any(|d| d.kind == ProviderKind::Claude);
    let has_codex = detected.iter().any(|d| d.kind == ProviderKind::Codex);
    let has_gemini = detected.iter().any(|d| d.kind == ProviderKind::Gemini);

    let presets: Vec<selector::PresetOption> = PipelinePreset::all()
        .iter()
        .map(|p| selector::PresetOption {
            name: p.to_string(),
            quality_bar: p.quality_bar().to_string(),
            speed_bar: p.speed_bar().to_string(),
            description: p.description(has_claude, has_codex, has_gemini),
        })
        .collect();

    let current = PipelinePreset::all()
        .iter()
        .position(|p| *p == config.pipeline_preset)
        .unwrap_or(1);

    if let Some(idx) = selector::preset_panel(&presets, current)? {
        let chosen = PipelinePreset::all()[idx];

        // Warn if switching away from Custom to a preset (overrides manual config)
        if config.pipeline_preset == PipelinePreset::Custom
            && chosen != PipelinePreset::Custom
            && !selector::confirm_action("This will override your custom agent configuration.")?
        {
            return Ok(());
        }

        config.pipeline_preset = chosen;
        if chosen != PipelinePreset::Custom {
            presets::apply_preset(chosen, &mut config.agents, detected);
        }
    }

    Ok(())
}

// --- Agent models ---

fn agent_model_display(agent: &AgentConfig) -> String {
    if let Some(model) = &agent.model {
        format!("{}:{}", agent.provider, model)
    } else if agent.provider == "default" {
        "default".to_string()
    } else {
        agent.provider.clone()
    }
}

fn agent_models_summary(agents: &AgentsConfig) -> String {
    let configs = agent_configs(agents);
    let all_default = configs
        .iter()
        .all(|ac| ac.provider == "default" && ac.model.is_none());
    if all_default {
        "all default".to_string()
    } else {
        let mut models: Vec<String> = configs.iter().map(|ac| agent_model_display(ac)).collect();
        models.sort();
        models.dedup();
        if models.len() <= 3 {
            models.join(", ")
        } else {
            format!("{} models configured", models.len())
        }
    }
}

/// Build the list of model choices: "default", then "provider:model" for ALL providers.
/// Returns (choice_label, is_available) pairs.
fn build_model_choices(detected: &[crate::provider::DetectedProvider]) -> Vec<(String, bool)> {
    use crate::provider::ProviderKind;
    let detected_names: Vec<&str> = detected.iter().map(|p| p.kind.cli_name()).collect();
    let all_kinds = [
        ProviderKind::Claude,
        ProviderKind::Codex,
        ProviderKind::Gemini,
    ];

    let mut choices = vec![("default".to_string(), true)];
    for kind in all_kinds {
        let installed = detected_names.contains(&kind.cli_name());
        // Add bare provider (uses provider's default model)
        choices.push((kind.cli_name().to_string(), installed));
        // Add provider:model for each known model
        for model in kind.available_models() {
            choices.push((format!("{}:{}", kind.cli_name(), model), installed));
        }
    }
    choices
}

/// Parse "provider:model" into (provider, Option<model>)
fn parse_model_choice(choice: &str) -> (&str, Option<&str>) {
    if choice == "default" {
        ("default", None)
    } else if let Some((provider, model)) = choice.split_once(':') {
        (provider, Some(model))
    } else {
        (choice, None)
    }
}

fn edit_agent_models(
    config: &mut config::Config,
    detected: &[crate::provider::DetectedProvider],
) -> Result<()> {
    let mut choices_with_avail = build_model_choices(detected);

    // If any agent uses a model not in the known list (e.g., deprecated model),
    // add it to choices so users can see it and migrate away from it.
    for ac in agent_configs(&config.agents) {
        let display = agent_model_display(ac);
        if !choices_with_avail
            .iter()
            .any(|(label, _)| label == &display)
            && display != "default"
        {
            // Deprecated/unknown model — show it but mark as unavailable
            choices_with_avail.push((display, false));
        }
    }

    let choice_labels: Vec<String> = choices_with_avail.iter().map(|(s, _)| s.clone()).collect();
    let choice_refs: Vec<&str> = choice_labels.iter().map(|s| s.as_str()).collect();
    let available: Vec<bool> = choices_with_avail.iter().map(|(_, a)| *a).collect();

    let labels: Vec<&str> = AGENT_ENTRIES.iter().map(|(_, label)| *label).collect();

    // Map current agent config to choice indices — always finds a match now
    let mut values: Vec<usize> = AGENT_ENTRIES
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let agent = &agent_configs(&config.agents)[i];
            let display = agent_model_display(agent);
            choice_refs.iter().position(|&c| c == display).unwrap_or(0)
        })
        .collect();

    if selector::toggle_list(&labels, &mut values, &choice_refs, &available)? {
        for (i, &val_idx) in values.iter().enumerate() {
            let (provider, model) = parse_model_choice(&choice_labels[val_idx]);
            set_agent_provider(&mut config.agents, i, provider);
            set_agent_model(&mut config.agents, i, model);
        }
    }

    Ok(())
}

// --- Disabled agents ---

fn disabled_agents_summary(agents: &AgentsConfig) -> String {
    let count = DISABLEABLE_AGENTS
        .iter()
        .filter(|&&i| !get_agent_enabled(agents, i))
        .count();
    if count == 0 {
        "none".to_string()
    } else {
        format!("{} disabled", count)
    }
}

fn edit_disabled_agents(config: &mut config::Config) -> Result<()> {
    let options: Vec<&str> = DISABLEABLE_AGENTS
        .iter()
        .map(|&i| AGENT_ENTRIES[i].1)
        .collect();

    // Pre-select the currently disabled agents
    let preselected: Vec<usize> = DISABLEABLE_AGENTS
        .iter()
        .enumerate()
        .filter(|(_, &agent_idx)| !get_agent_enabled(&config.agents, agent_idx))
        .map(|(menu_idx, _)| menu_idx)
        .collect();

    let selected = selector::multi_select(
        "Disabled agents (toggle to disable):",
        &options,
        &preselected,
    )?;

    // Apply: selected means disabled
    for (menu_idx, &agent_idx) in DISABLEABLE_AGENTS.iter().enumerate() {
        let disabled = selected.contains(&menu_idx);
        set_agent_enabled(&mut config.agents, agent_idx, !disabled);
    }

    Ok(())
}

// --- Checkpoints ---

fn edit_checkpoints(config: &mut config::Config) -> Result<()> {
    let options = [
        "After researcher",
        "After review loop",
        "After planner",
        "After implementation",
    ];
    let current: Vec<usize> = config
        .checkpoints
        .iter()
        .map(|c| match c {
            Checkpoint::AfterResearcher => 0,
            Checkpoint::AfterReviewLoop => 1,
            Checkpoint::AfterPlanner => 2,
            Checkpoint::AfterImplementation => 3,
        })
        .collect();
    let selected = selector::multi_select("Checkpoints (approval gates):", &options, &current)?;
    config.checkpoints = selected
        .iter()
        .filter_map(|&i| match i {
            0 => Some(Checkpoint::AfterResearcher),
            1 => Some(Checkpoint::AfterReviewLoop),
            2 => Some(Checkpoint::AfterPlanner),
            3 => Some(Checkpoint::AfterImplementation),
            _ => None,
        })
        .collect();
    Ok(())
}

// --- Branch strategy ---

fn edit_branch_strategy(config: &mut config::Config) -> Result<()> {
    let options = [
        "separate (per task)",
        "single (feature branch)",
        "planner decides",
    ];
    let current = match config.implementation.branch_strategy {
        BranchStrategy::Separate => 0,
        BranchStrategy::Single => 1,
        BranchStrategy::PlannerDecides => 2,
    };
    let idx = selector::select("Branch strategy:", &options, current)?;
    config.implementation.branch_strategy = match idx {
        0 => BranchStrategy::Separate,
        1 => BranchStrategy::Single,
        _ => BranchStrategy::PlannerDecides,
    };
    Ok(())
}

// --- Parallel limit ---

fn edit_parallel_limit(config: &mut config::Config) -> Result<()> {
    let options = ["1", "2", "4", "8"];
    let current = options
        .iter()
        .position(|&o| o == config.implementation.parallel_limit.to_string())
        .unwrap_or(2);
    let idx = selector::select("Parallel implementation limit:", &options, current)?;
    config.implementation.parallel_limit = options[idx].parse().unwrap_or(4);
    Ok(())
}

// --- Helpers: agent config access by index ---

fn agent_configs(agents: &AgentsConfig) -> Vec<&AgentConfig> {
    vec![
        &agents.researcher,
        &agents.plan_reviewer,
        &agents.plan_security_auditor,
        &agents.judge,
        &agents.planner,
        &agents.test_architect,
        &agents.implementor,
        &agents.code_reviewer,
        &agents.code_security_auditor,
        &agents.validator,
    ]
}

fn set_agent_provider(agents: &mut AgentsConfig, idx: usize, provider: &str) {
    let agent = match idx {
        0 => &mut agents.researcher,
        1 => &mut agents.plan_reviewer,
        2 => &mut agents.plan_security_auditor,
        3 => &mut agents.judge,
        4 => &mut agents.planner,
        5 => &mut agents.test_architect,
        6 => &mut agents.implementor,
        7 => &mut agents.code_reviewer,
        8 => &mut agents.code_security_auditor,
        9 => &mut agents.validator,
        _ => return,
    };
    agent.provider = provider.to_string();
}

fn get_agent_enabled(agents: &AgentsConfig, idx: usize) -> bool {
    match idx {
        0 => agents.researcher.enabled,
        1 => agents.plan_reviewer.enabled,
        2 => agents.plan_security_auditor.enabled,
        3 => agents.judge.enabled,
        4 => agents.planner.enabled,
        5 => agents.test_architect.enabled,
        6 => agents.implementor.enabled,
        7 => agents.code_reviewer.enabled,
        8 => agents.code_security_auditor.enabled,
        9 => agents.validator.enabled,
        _ => true,
    }
}

fn set_agent_enabled(agents: &mut AgentsConfig, idx: usize, enabled: bool) {
    let agent = match idx {
        0 => &mut agents.researcher,
        1 => &mut agents.plan_reviewer,
        2 => &mut agents.plan_security_auditor,
        3 => &mut agents.judge,
        4 => &mut agents.planner,
        5 => &mut agents.test_architect,
        6 => &mut agents.implementor,
        7 => &mut agents.code_reviewer,
        8 => &mut agents.code_security_auditor,
        9 => &mut agents.validator,
        _ => return,
    };
    agent.enabled = enabled;
}

fn set_agent_model(agents: &mut AgentsConfig, idx: usize, model: Option<&str>) {
    let agent = match idx {
        0 => &mut agents.researcher,
        1 => &mut agents.plan_reviewer,
        2 => &mut agents.plan_security_auditor,
        3 => &mut agents.judge,
        4 => &mut agents.planner,
        5 => &mut agents.test_architect,
        6 => &mut agents.implementor,
        7 => &mut agents.code_reviewer,
        8 => &mut agents.code_security_auditor,
        9 => &mut agents.validator,
        _ => return,
    };
    agent.model = model.map(|m| m.to_string());
}

// --- Label helpers ---

fn checkpoint_label(c: &Checkpoint) -> &'static str {
    match c {
        Checkpoint::AfterResearcher => "after researcher",
        Checkpoint::AfterReviewLoop => "after review loop",
        Checkpoint::AfterPlanner => "after planner",
        Checkpoint::AfterImplementation => "after implementation",
    }
}

fn branch_label(b: &BranchStrategy) -> &'static str {
    match b {
        BranchStrategy::Separate => "separate",
        BranchStrategy::Single => "single",
        BranchStrategy::PlannerDecides => "planner decides",
    }
}
