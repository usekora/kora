use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use std::io::{self, Write};


pub struct Renderer {
    stdout: io::Stdout,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
        }
    }

    pub fn stage_header(&mut self, name: &str, status: &str) {
        let dots = ".".repeat(50usize.saturating_sub(name.len() + status.len()));

        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(name),
            SetAttribute(Attribute::Reset),
            Print(" "),
            SetForegroundColor(Color::DarkGrey),
            Print(dots),
            Print(" "),
            ResetColor,
            Print(status),
            Print(" "),
            SetForegroundColor(Color::Green),
            Print("●"),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }

    pub fn stage_complete(&mut self, name: &str, duration_secs: u64) {
        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(Color::Green),
            Print("✓ "),
            ResetColor,
            Print(name),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  {}s", duration_secs)),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }

    pub fn finding(&mut self, severity: &str, text: &str) {
        let (glyph, color) = match severity.to_uppercase().as_str() {
            "HIGH" => ("▲", Color::Red),
            "MEDIUM" | "MED" => ("■", Color::Yellow),
            _ => ("·", Color::DarkGrey),
        };

        execute!(
            self.stdout,
            Print("    "),
            SetForegroundColor(color),
            Print(glyph),
            Print(" "),
            Print(severity.to_uppercase()),
            ResetColor,
            Print(format!("   {}\n", text)),
        )
        .ok();
    }

    pub fn verdict_line(&mut self, title: &str, accepted: bool, reason: &str) {
        let (glyph, color) = if accepted {
            ("▲", Color::Red)
        } else {
            ("·", Color::DarkGrey)
        };

        let status = if accepted { "accepted" } else { "dismissed" };

        execute!(
            self.stdout,
            Print("    "),
            SetForegroundColor(color),
            Print(glyph),
            ResetColor,
            Print(format!(" {:<25} {} — {}\n", title, status, reason)),
        )
        .ok();
    }

    pub fn separator(&mut self) {
        execute!(
            self.stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"),
            ResetColor,
        )
        .ok();
    }

    pub fn info(&mut self, text: &str) {
        execute!(
            self.stdout,
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  {}\n", text)),
            ResetColor,
        )
        .ok();
    }

    pub fn text(&mut self, text: &str) {
        execute!(self.stdout, Print(format!("  {}\n", text)),).ok();
    }

    pub fn welcome(&mut self, version: &str, provider: &str, checkpoints: usize) {
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(format!("  kora v{}", version)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::DarkGrey),
            Print(format!(
                " · {} (default) · {} checkpoints configured",
                provider, checkpoints
            )),
            ResetColor,
            Print("\n\n"),
            Print("  ready. describe what you'd like to build, fix, or change.\n\n"),
        )
        .ok();
    }

    pub fn checkpoint_prompt(&mut self, next_stage: &str) -> bool {
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::Yellow),
            Print("  ■ checkpoint"),
            ResetColor,
            Print(format!(": approve to proceed to {}? [y/n] ", next_stage)),
        )
        .ok();
        self.stdout.flush().ok();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            return false;
        }
        let answer = input.trim().to_lowercase();
        answer == "y" || answer == "yes"
    }

    pub fn review_loop_summary(
        &mut self,
        iteration: u32,
        valid: u32,
        dismissed: u32,
        overall: &str,
    ) {
        let color = if overall == "APPROVE" {
            Color::Green
        } else {
            Color::Yellow
        };

        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(color),
            Print(format!(
                "review iteration {}: {} valid, {} dismissed",
                iteration, valid, dismissed
            )),
            ResetColor,
            Print(format!(" → {}\n", overall)),
        )
        .ok();
    }

    pub fn escalation(&mut self, message: &str) {
        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(Color::Red),
            SetAttribute(Attribute::Bold),
            Print("▲ escalation"),
            SetAttribute(Attribute::Reset),
            ResetColor,
            Print(format!(": {}\n", message)),
        )
        .ok();
    }

    pub fn iteration_header(&mut self, iteration: u32, max: u32) {
        let label = format!("review loop · iteration {} of {}", iteration, max);
        self.stage_header(&label, "running");
    }

    pub fn implementation_complete(&mut self, total_tasks: usize, total_duration_secs: u64) {
        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(Color::Green),
            Print(format!(
                "all {} tasks complete in {}s",
                total_tasks, total_duration_secs
            )),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }

    pub fn task_failure(&mut self, task_id: &str, error: &str) {
        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(Color::Red),
            Print(format!("task {} failed: {}", task_id, error)),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }

    pub fn validation_result(
        &mut self,
        passed: bool,
        blocking: u32,
        minor: u32,
        tests_passed: u32,
        tests_failed: u32,
    ) {
        let (icon, color) = if passed {
            ("✓", Color::Green)
        } else {
            ("✗", Color::Red)
        };

        execute!(
            self.stdout,
            Print("\n  "),
            SetForegroundColor(color),
            Print(format!("{} validation {}", icon, if passed { "passed" } else { "failed" })),
            ResetColor,
            Print(format!(
                "  blocking: {}  minor: {}  tests: {} passed, {} failed\n",
                blocking, minor, tests_passed, tests_failed
            )),
        )
        .ok();
    }

    pub fn merge_info(&mut self, message: &str) {
        execute!(
            self.stdout,
            Print("  "),
            SetForegroundColor(Color::Cyan),
            Print("↳ "),
            ResetColor,
            Print(format!("{}\n", message)),
        )
        .ok();
    }

    pub fn run_complete(&mut self, run_id: &str) {
        execute!(
            self.stdout,
            Print("\n"),
            SetForegroundColor(Color::Green),
            SetAttribute(Attribute::Bold),
            Print("  ✓ run complete"),
            SetAttribute(Attribute::Reset),
            ResetColor,
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  ({})", run_id)),
            ResetColor,
            Print("\n"),
        )
        .ok();
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
