use std::collections::HashMap;
use std::io::{self, Write};

use crossterm::{
    cursor, execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal,
};

use crate::pipeline::implementation::{ImplementationTaskStatus, TaskState};

pub struct Dashboard {
    stdout: io::Stdout,
    task_order: Vec<String>,
    last_render_lines: u16,
    total_tasks: usize,
    show_task: Option<String>,
}

impl Dashboard {
    pub fn new(task_order: Vec<String>) -> Self {
        let total = task_order.len();
        Self {
            stdout: io::stdout(),
            task_order,
            last_render_lines: 0,
            total_tasks: total,
            show_task: None,
        }
    }

    pub fn render(&mut self, task_states: &HashMap<String, TaskState>) {
        if self.last_render_lines > 0 {
            execute!(
                self.stdout,
                cursor::MoveUp(self.last_render_lines),
                terminal::Clear(terminal::ClearType::FromCursorDown),
            )
            .ok();
        }

        let completed = task_states
            .values()
            .filter(|s| matches!(s.status, ImplementationTaskStatus::Complete { .. }))
            .count();

        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print("  implementing"),
            SetAttribute(Attribute::Reset),
            Print(" "),
            SetForegroundColor(Color::DarkGrey),
            Print(".............................."),
            Print(" "),
            ResetColor,
            Print(format!("{} of {} ", completed, self.total_tasks)),
            SetForegroundColor(Color::Green),
            Print("●"),
            ResetColor,
            Print("\n\n"),
        )
        .ok();

        let mut lines: u16 = 3;

        for task_id in &self.task_order {
            if let Some(state) = task_states.get(task_id) {
                render_task_line(&mut self.stdout, state);
                lines += 1;
            }
        }

        execute!(self.stdout, Print("\n")).ok();
        lines += 1;

        self.last_render_lines = lines;
        self.stdout.flush().ok();
    }

    pub fn set_show_task(&mut self, task_id: Option<String>) {
        self.show_task = task_id;
    }

    pub fn showing_task(&self) -> Option<&str> {
        self.show_task.as_deref()
    }

    pub fn total_tasks(&self) -> usize {
        self.total_tasks
    }

    pub fn task_order(&self) -> &[String] {
        &self.task_order
    }
}

fn render_task_line(stdout: &mut io::Stdout, state: &TaskState) {
    let id = &state.task.id;
    let branch = &state.branch_name;

    let (status_text, color, bar) = match &state.status {
        ImplementationTaskStatus::Pending => {
            ("pending".to_string(), Color::DarkGrey, render_bar(0))
        }
        ImplementationTaskStatus::Blocked { waiting_on } => {
            let deps = waiting_on.join(",");
            (
                format!("blocked -> {}", deps),
                Color::DarkGrey,
                render_bar(0),
            )
        }
        ImplementationTaskStatus::Running { provider, .. } => (
            format!("running  {}", provider),
            Color::Cyan,
            render_bar(50),
        ),
        ImplementationTaskStatus::Complete {
            duration_secs,
            files_changed,
        } => (
            format!("done {}s  {} files", duration_secs, files_changed),
            Color::Green,
            render_bar(100),
        ),
        ImplementationTaskStatus::Failed { attempts, .. } => (
            format!("FAILED (attempt {})", attempts),
            Color::Red,
            render_bar(0),
        ),
        ImplementationTaskStatus::Conflict { .. } => {
            ("CONFLICT".to_string(), Color::Yellow, render_bar(0))
        }
    };

    execute!(
        stdout,
        Print("    "),
        SetForegroundColor(color),
        Print(format!("{:<4}", id)),
        ResetColor,
        Print(format!(" {} ", bar)),
        SetForegroundColor(color),
        Print(format!("{:<30}", status_text)),
        ResetColor,
        SetForegroundColor(Color::DarkGrey),
        Print(format!(" {}", branch)),
        ResetColor,
        Print("\n"),
    )
    .ok();
}

pub fn render_bar(percent: u8) -> String {
    let filled = (percent as usize * 12) / 100;
    let empty = 12 - filled;
    let bar: String = std::iter::repeat_n('█', filled)
        .chain(std::iter::repeat_n('░', empty))
        .collect();
    bar
}
