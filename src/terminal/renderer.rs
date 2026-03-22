use crossterm::{
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
};
use std::io;


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
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
