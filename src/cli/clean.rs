use anyhow::Result;
use chrono::Utc;
use std::path::Path;

use crate::git::worktree::WorktreeManager;
use crate::state::{RunState, Stage};
use crate::terminal::selector;
use crate::terminal::Renderer;

pub async fn run_clean(project_root: &Path) -> Result<()> {
    let runs_dir = crate::config::runs_dir();
    let mut renderer = Renderer::new();

    if !runs_dir.exists() {
        renderer.info("no run data to clean");
        return Ok(());
    }

    let all_runs = load_cleanable_runs(&runs_dir)?;

    if all_runs.is_empty() {
        renderer.info("no completed or failed runs to clean");
        return Ok(());
    }

    let total_size = calculate_dir_size(&runs_dir);
    let size_display = format_size(total_size);

    renderer.text(&format!(
        "{} completed/failed runs using {}",
        all_runs.len(),
        size_display
    ));
    renderer.text("");

    let options = [
        "All completed/failed runs",
        "Older than 1 week",
        "Pick specific ones",
        "Cancel",
    ];

    let choice = selector::select("Clean up:", &options)?;

    let runs_to_clean: Vec<&RunState> = match choice {
        0 => all_runs.iter().collect(),
        1 => {
            let cutoff = Utc::now() - chrono::Duration::days(7);
            all_runs.iter().filter(|r| r.created_at < cutoff).collect()
        }
        2 => {
            let labels: Vec<String> = all_runs
                .iter()
                .map(|r| {
                    let request = truncate_request(&r.request, 40);
                    let size = calculate_dir_size(&runs_dir.join(&r.id));
                    format!(
                        "\"{}\" · {} · {}",
                        request,
                        r.status.label(),
                        format_size(size)
                    )
                })
                .collect();
            let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
            let selected = selector::multi_select("Select runs to clean:", &label_refs)?;
            selected.iter().filter_map(|&i| all_runs.get(i)).collect()
        }
        _ => {
            renderer.info("clean cancelled");
            return Ok(());
        }
    };

    if runs_to_clean.is_empty() {
        renderer.info("no runs match the criteria");
        return Ok(());
    }

    let mut cleaned = 0u32;
    for run in &runs_to_clean {
        let run_path = runs_dir.join(&run.id);
        if run_path.exists() {
            std::fs::remove_dir_all(&run_path)?;
            cleaned += 1;
        }
    }

    let wt_manager = WorktreeManager::new(project_root);
    wt_manager.cleanup_all().await?;

    renderer.stage_complete(&format!("cleaned {} runs, worktrees pruned", cleaned), 0);

    Ok(())
}

fn load_cleanable_runs(runs_dir: &Path) -> Result<Vec<RunState>> {
    let mut runs = Vec::new();
    if !runs_dir.exists() {
        return Ok(runs);
    }

    for entry in std::fs::read_dir(runs_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let run_id = entry.file_name().to_string_lossy().to_string();
            if let Ok(state) = RunState::load(runs_dir, &run_id) {
                match state.status {
                    Stage::Complete | Stage::Failed(_) => runs.push(state),
                    _ => {}
                }
            }
        }
    }

    runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(runs)
}

fn calculate_dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .filter(|m| m.is_file())
        .map(|m| m.len())
        .sum()
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn truncate_request(request: &str, max_len: usize) -> String {
    let first_line = request.lines().next().unwrap_or(request);
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len])
    } else {
        first_line.to_string()
    }
}
