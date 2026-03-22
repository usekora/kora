use anyhow::Result;
use std::path::Path;

use crate::config::{self, BranchStrategy, Verbosity};
use crate::provider::detect_providers;
use crate::state::Checkpoint;
use crate::terminal::selector;

pub fn run_configure(project_root: &Path) -> Result<()> {
    let mut config = config::load(project_root)?;

    let detected = detect_providers();
    if detected.is_empty() {
        eprintln!("  No AI CLI tools detected. Install claude or codex first.");
        return Ok(());
    }

    let provider_names: Vec<&str> = detected.iter().map(|p| p.kind.cli_name()).collect();

    let default_idx = selector::select("Default AI provider:", &provider_names)?;
    config.default_provider = provider_names[default_idx].to_string();

    let assign_idx = selector::select(
        "Assign providers per agent role, or use default for all?",
        &["Use default for all", "Assign per role"],
    )?;

    if assign_idx == 1 {
        let roles = [
            "researcher",
            "reviewer",
            "security_auditor",
            "judge",
            "planner",
            "test_architect",
            "implementor",
            "validator",
        ];
        let mut options: Vec<&str> = vec!["default"];
        options.extend(&provider_names);

        for role in roles {
            let idx = selector::select(&format!("Provider for {}:", role), &options)?;
            let provider = options[idx].to_string();
            match role {
                "researcher" => config.agents.researcher.provider = provider,
                "reviewer" => config.agents.reviewer.provider = provider,
                "security_auditor" => config.agents.security_auditor.provider = provider,
                "judge" => config.agents.judge.provider = provider,
                "planner" => config.agents.planner.provider = provider,
                "test_architect" => config.agents.test_architect.provider = provider,
                "implementor" => config.agents.implementor.provider = provider,
                "validator" => config.agents.validator.provider = provider,
                _ => {}
            }
        }
    }

    let checkpoint_options = [
        "After researcher proposes direction",
        "After each review/judge iteration",
        "After planner breaks down tasks",
        "After implementation completes",
    ];
    let checkpoint_selected = selector::multi_select(
        "Which stages require your approval before proceeding?",
        &checkpoint_options,
    )?;

    config.checkpoints = checkpoint_selected
        .iter()
        .filter_map(|&i| match i {
            0 => Some(Checkpoint::AfterResearcher),
            1 => Some(Checkpoint::AfterReviewLoop),
            2 => Some(Checkpoint::AfterPlanner),
            3 => Some(Checkpoint::AfterImplementation),
            _ => None,
        })
        .collect();

    let branch_idx = selector::select(
        "Default branch strategy for implementation:",
        &[
            "Separate branch per task",
            "Single feature branch",
            "Let planner decide",
        ],
    )?;
    config.implementation.branch_strategy = match branch_idx {
        0 => BranchStrategy::Separate,
        1 => BranchStrategy::Single,
        _ => BranchStrategy::PlannerDecides,
    };

    let verbosity_idx = selector::select(
        "Default output verbosity:",
        &["focused", "detailed", "verbose"],
    )?;
    config.output.default_verbosity = match verbosity_idx {
        0 => Verbosity::Focused,
        1 => Verbosity::Detailed,
        _ => Verbosity::Verbose,
    };

    config::save_home(&config)?;
    println!("\n  Configuration saved to ~/.kora/config.yml\n");

    Ok(())
}
