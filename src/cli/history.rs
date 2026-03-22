use anyhow::Result;
use chrono::Utc;
use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use std::io::{self, Write};
use std::path::Path;

use crate::state::{RunState, Stage};
use crate::terminal::selector;

struct RunGroup {
    label: String,
    runs: Vec<RunState>,
}

pub fn run_history(project_root: &Path) -> Result<()> {
    let config = crate::config::load(project_root)?;
    let runs_dir = project_root.join(&config.runs_dir);

    if !runs_dir.exists() {
        println!("  no run history found");
        return Ok(());
    }

    let mut all_runs = load_all_runs(&runs_dir)?;

    if all_runs.is_empty() {
        println!("  no run history found");
        return Ok(());
    }

    all_runs.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let groups = group_runs_by_date(&all_runs);
    let mut stdout = io::stdout();

    execute!(stdout, Print("\n"))?;

    let mut flat_runs: Vec<&RunState> = Vec::new();

    for group in &groups {
        execute!(
            stdout,
            Print("  "),
            SetForegroundColor(Color::DarkGrey),
            Print(&group.label),
            ResetColor,
            Print("\n"),
        )?;

        for run in &group.runs {
            let icon = match &run.status {
                Stage::Complete => "✓",
                Stage::Failed(_) => "✗",
                _ => "●",
            };
            let color = match &run.status {
                Stage::Complete => Color::Green,
                Stage::Failed(_) => Color::Red,
                _ => Color::Yellow,
            };

            let request_display = truncate(&run.request, 40);
            let duration = format_duration(run);
            let status_info = match &run.status {
                Stage::Complete => duration,
                Stage::Failed(err) => {
                    let short_err = truncate(err, 30);
                    format!("failed: {}", short_err)
                }
                other => format!("interrupted at {}", other.label()),
            };

            execute!(
                stdout,
                Print("    "),
                SetForegroundColor(color),
                Print(icon),
                ResetColor,
                Print(format!(" \"{}\"", request_display)),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("  {}", status_info)),
                ResetColor,
                Print("\n"),
            )?;

            flat_runs.push(run);
        }

        execute!(stdout, Print("\n"))?;
    }

    if flat_runs.is_empty() {
        return Ok(());
    }

    let run_options: Vec<String> = flat_runs
        .iter()
        .map(|r| {
            let display = truncate(&r.request, 50);
            format!("{} · {}", r.id, display)
        })
        .collect();
    let option_refs: Vec<&str> = run_options.iter().map(|s| s.as_str()).collect();

    let idx = selector::select("Select a run to view details (esc to exit):", &option_refs)?;

    if idx < flat_runs.len() {
        print_run_detail(&mut stdout, flat_runs[idx])?;
    }

    Ok(())
}

fn load_all_runs(runs_dir: &Path) -> Result<Vec<RunState>> {
    let mut runs = Vec::new();
    if !runs_dir.exists() {
        return Ok(runs);
    }

    for entry in std::fs::read_dir(runs_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let run_id = entry.file_name().to_string_lossy().to_string();
            if let Ok(state) = RunState::load(runs_dir, &run_id) {
                runs.push(state);
            }
        }
    }

    Ok(runs)
}

fn group_runs_by_date(runs: &[RunState]) -> Vec<RunGroup> {
    let now = Utc::now();
    let today = now.date_naive();
    let yesterday = today.pred_opt().unwrap_or(today);

    let mut today_runs = Vec::new();
    let mut yesterday_runs = Vec::new();
    let mut older: std::collections::BTreeMap<String, Vec<RunState>> =
        std::collections::BTreeMap::new();

    for run in runs {
        let run_date = run.created_at.date_naive();
        if run_date == today {
            today_runs.push(run.clone());
        } else if run_date == yesterday {
            yesterday_runs.push(run.clone());
        } else {
            let label = run_date.format("%Y-%m-%d").to_string();
            older.entry(label).or_default().push(run.clone());
        }
    }

    let mut groups = Vec::new();

    if !today_runs.is_empty() {
        groups.push(RunGroup {
            label: "today".to_string(),
            runs: today_runs,
        });
    }
    if !yesterday_runs.is_empty() {
        groups.push(RunGroup {
            label: "yesterday".to_string(),
            runs: yesterday_runs,
        });
    }
    for (label, runs) in older.into_iter().rev() {
        groups.push(RunGroup { label, runs });
    }

    groups
}

fn print_run_detail(stdout: &mut io::Stdout, run: &RunState) -> Result<()> {
    execute!(
        stdout,
        Print("\n"),
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print(format!("  run {}", run.id)),
        SetAttribute(Attribute::Reset),
        ResetColor,
        Print("\n"),
    )?;

    execute!(
        stdout,
        Print(format!("  request: \"{}\"\n", run.request)),
        Print(format!("  status:  {}\n", run.status.label())),
        Print(format!("  created: {}\n", run.created_at.format("%Y-%m-%d %H:%M:%S UTC"))),
        Print(format!("  updated: {}\n", run.updated_at.format("%Y-%m-%d %H:%M:%S UTC"))),
    )?;

    if let Some(ref err) = run.error {
        execute!(
            stdout,
            SetForegroundColor(Color::Red),
            Print(format!("  error:   {}\n", err)),
            ResetColor,
        )?;
    }

    if !run.timestamps.is_empty() {
        execute!(stdout, Print("\n  stages:\n"))?;
        let mut stages: Vec<_> = run.timestamps.iter().collect();
        stages.sort_by(|a, b| a.1.cmp(b.1));
        for (stage, ts) in stages {
            execute!(
                stdout,
                SetForegroundColor(Color::DarkGrey),
                Print(format!("    {} — {}\n", stage, ts.format("%H:%M:%S"))),
                ResetColor,
            )?;
        }
    }

    execute!(stdout, Print("\n"))?;
    stdout.flush()?;

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    let first_line = s.lines().next().unwrap_or(s);
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len])
    } else {
        first_line.to_string()
    }
}

fn format_duration(run: &RunState) -> String {
    let duration = run.updated_at.signed_duration_since(run.created_at);
    let secs = duration.num_seconds();
    if secs < 60 {
        format!("{}s", secs)
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}
