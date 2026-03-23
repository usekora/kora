use crossterm::{
    cursor, execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io::{self, Write};

pub struct Renderer {
    stdout: io::Stdout,
}

/// A background spinner that animates on the stage header line.
pub struct Spinner {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    status: std::sync::Arc<std::sync::Mutex<String>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

impl Spinner {
    pub fn start(row: u16, name: &str, dot_count: usize, initial_status: &str, provider_info: &str) -> Self {
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let status = std::sync::Arc::new(std::sync::Mutex::new(initial_status.to_string()));
        let stop_clone = stop.clone();
        let status_clone = status.clone();
        let name = name.to_string();
        let provider_info = provider_info.to_string();

        let handle = std::thread::spawn(move || {
            let mut stdout = io::stdout();
            let mut tick = 0usize;
            let wave_width = 3usize;
            let start_time = std::time::Instant::now();

            execute!(stdout, cursor::Hide).ok();

            while !stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                let status_text = status_clone.lock().unwrap().clone();
                let frame = SPINNER_FRAMES[tick % SPINNER_FRAMES.len()];
                let elapsed = start_time.elapsed();
                let secs = elapsed.as_secs();
                let timer = format!("{}:{:02}", secs / 60, secs % 60);

                // Animated dots: wave of bright purple dots moving across
                let wave_pos = tick % (dot_count + wave_width);
                let mut dots = String::new();
                for i in 0..dot_count {
                    if i >= wave_pos.saturating_sub(wave_width) && i < wave_pos {
                        dots.push_str("\x1b[38;2;108;92;231m.\x1b[0m");
                    } else {
                        dots.push_str("\x1b[38;2;80;80;80m.\x1b[0m");
                    }
                }

                execute!(
                    stdout,
                    cursor::MoveTo(0, row),
                    Clear(ClearType::CurrentLine),
                    Print("  "),
                    SetForegroundColor(Color::White),
                    SetAttribute(Attribute::Bold),
                    Print(&name),
                    SetAttribute(Attribute::Reset),
                    Print(" "),
                    ResetColor,
                ).ok();
                write!(stdout, "{}", dots).ok();
                execute!(stdout, Print(" "), SetForegroundColor(Color::Cyan), Print(&status_text)).ok();
                if !provider_info.is_empty() {
                    execute!(stdout,
                        SetForegroundColor(Color::Rgb { r: 130, g: 130, b: 130 }),
                        Print(format!(" ({})", provider_info)),
                    ).ok();
                }
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb { r: 130, g: 130, b: 130 }),
                    Print(format!(" {}", timer)),
                    Print(" "),
                    SetForegroundColor(Color::Green),
                    Print(frame),
                    ResetColor,
                ).ok();
                stdout.flush().ok();
                tick += 1;
                std::thread::sleep(std::time::Duration::from_millis(80));
            }

            // Final: elapsed time, green status with checkmark
            let status_text = status_clone.lock().unwrap().clone();
            let elapsed = start_time.elapsed();
            let secs = elapsed.as_secs();
            let timer = format!("{}:{:02}", secs / 60, secs % 60);
            let dim_dots = format!("\x1b[38;2;80;80;80m{}\x1b[0m", ".".repeat(dot_count));
            execute!(
                stdout,
                cursor::MoveTo(0, row),
                Clear(ClearType::CurrentLine),
                Print("  "),
                SetForegroundColor(Color::White),
                SetAttribute(Attribute::Bold),
                Print(&name),
                SetAttribute(Attribute::Reset),
                Print(" "),
                ResetColor,
            ).ok();
            write!(stdout, "{}", dim_dots).ok();
            execute!(stdout, Print(" "), SetForegroundColor(Color::Green), Print(&status_text)).ok();
            if !provider_info.is_empty() {
                execute!(stdout,
                    SetForegroundColor(Color::Rgb { r: 130, g: 130, b: 130 }),
                    Print(format!(" ({})", provider_info)),
                ).ok();
            }
            execute!(
                stdout,
                SetForegroundColor(Color::Rgb { r: 130, g: 130, b: 130 }),
                Print(format!(" {}", timer)),
                Print(" "),
                SetForegroundColor(Color::Green),
                Print("✓"),
                ResetColor,
                cursor::Show,
            ).ok();
            stdout.flush().ok();
        });

        Self { stop, status, handle: Some(handle) }
    }

    /// Update the status text shown next to the spinner.
    pub fn set_status(&self, new_status: &str) {
        if let Ok(mut s) = self.status.lock() {
            *s = new_status.to_string();
        }
    }

    pub fn stop(mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            h.join().ok();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            stdout: io::stdout(),
        }
    }

    /// Start a stage with an animated spinner. `provider_info` is e.g. "claude:sonnet-4-6".
    pub fn stage_header(&mut self, name: &str, status: &str) -> Spinner {
        self.stage_header_with_provider(name, status, None)
    }

    pub fn stage_header_with_provider(&mut self, name: &str, status: &str, provider_info: Option<&str>) -> Spinner {
        let dot_count = 40usize.saturating_sub(name.len());

        // Print initial line
        execute!(
            self.stdout,
            Print("\r\n  "),
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(name),
            SetAttribute(Attribute::Reset),
            Print(" "),
            SetForegroundColor(Color::Rgb { r: 130, g: 130, b: 130 }),
            Print(".".repeat(dot_count)),
            Print(" "),
            ResetColor,
            Print(status),
            Print(" "),
        ).ok();
        self.stdout.flush().ok();

        // Get cursor position with raw mode briefly enabled
        crossterm::terminal::enable_raw_mode().ok();
        let (_, row) = cursor::position().unwrap_or((0, 0));
        crossterm::terminal::disable_raw_mode().ok();

        execute!(self.stdout, Print("\r\n")).ok();

        Spinner::start(row, name, dot_count, status, provider_info.unwrap_or(""))
    }

    pub fn stage_complete(&mut self, name: &str, duration_secs: u64) {
        execute!(
            self.stdout,
            Print("\r\n  "),
            SetForegroundColor(Color::Green),
            Print("✓ "),
            ResetColor,
            Print(name),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(format!("  {}s", duration_secs)),
            ResetColor,
            Print("\r\n"),
        )
        .ok();
    }

    pub fn finding(&mut self, severity: &str, text: &str) {
        let (glyph, color) = match severity.to_uppercase().as_str() {
            "HIGH" => ("▲", Color::Red),
            "MEDIUM" | "MED" => ("■", Color::Yellow),
            _ => (
                "·",
                Color::Rgb {
                    r: 130,
                    g: 130,
                    b: 130,
                },
            ),
        };

        execute!(
            self.stdout,
            Print("    "),
            SetForegroundColor(color),
            Print(glyph),
            Print(" "),
            Print(severity.to_uppercase()),
            ResetColor,
            Print(format!("   {}\r\n", text)),
        )
        .ok();
    }

    pub fn verdict_line(&mut self, title: &str, accepted: bool, reason: &str) {
        let (glyph, color) = if accepted {
            ("▲", Color::Red)
        } else {
            (
                "·",
                Color::Rgb {
                    r: 130,
                    g: 130,
                    b: 130,
                },
            )
        };

        let status = if accepted { "accepted" } else { "dismissed" };

        execute!(
            self.stdout,
            Print("    "),
            SetForegroundColor(color),
            Print(glyph),
            ResetColor,
            Print(format!(" {:<25} {} — {}\r\n", title, status, reason)),
        )
        .ok();
    }

    pub fn separator(&mut self) {
        execute!(
            self.stdout,
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\r\n"),
            ResetColor,
        )
        .ok();
    }

    pub fn info(&mut self, text: &str) {
        execute!(
            self.stdout,
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(format!("  {}\r\n", text)),
            ResetColor,
        )
        .ok();
    }

    pub fn text(&mut self, text: &str) {
        execute!(self.stdout, Print(format!("  {}\r\n", text)),).ok();
    }

    pub fn welcome(&mut self, version: &str, _provider: &str, project_path: &std::path::Path) {
        let project_name = project_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Kora strings art + title
        execute!(
            self.stdout,
            Print("\r\n"),
            SetForegroundColor(Color::Rgb {
                r: 108,
                g: 92,
                b: 231
            }),
            Print("   ╲ │ ╱\r\n"),
            Print("    ╲│╱   "),
            SetAttribute(Attribute::Bold),
            Print("KORA"),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(format!(" v{}\r\n", version)),
            SetForegroundColor(Color::Rgb {
                r: 108,
                g: 92,
                b: 231
            }),
            Print("     ●\r\n"),
            ResetColor,
        )
        .ok();

        // Project info
        execute!(
            self.stdout,
            Print("\r\n"),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print("   📂 "),
            ResetColor,
            Print(format!("{}\r\n", project_name)),
        )
        .ok();

        // Hints
        execute!(
            self.stdout,
            Print("\r\n"),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print("   Describe what you'd like to build, fix, or change.\r\n"),
            Print("   /help for commands · /configure to customize\r\n"),
            ResetColor,
            Print("\r\n"),
        )
        .ok();
    }

    /// Clear the separator + raw input line and reprint user input as a styled message.
    pub fn echo_input(&mut self, input: &str) {
        // Box was already cleared on submit, just print the echoed message
        execute!(
            self.stdout,
            SetForegroundColor(Color::White),
            SetAttribute(Attribute::Bold),
            Print(format!("  > {}", input)),
            SetAttribute(Attribute::Reset),
            ResetColor,
            Print("\r\n"),
        )
        .ok();
    }

    /// Show that a command was executed.
    pub fn command_result(&mut self, text: &str) {
        execute!(
            self.stdout,
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(format!("  {}\r\n", text)),
            ResetColor,
        )
        .ok();
    }

    /// Light separator between interactions.
    pub fn interaction_break(&mut self) {
        execute!(self.stdout, Print("\r\n")).ok();
    }

    pub fn checkpoint_prompt(&mut self, next_stage: &str) -> bool {
        execute!(
            self.stdout,
            Print("\r\n"),
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
            Print("\r\n  "),
            SetForegroundColor(color),
            Print(format!(
                "review iteration {}: {} valid, {} dismissed",
                iteration, valid, dismissed
            )),
            ResetColor,
            Print(format!(" → {}\r\n", overall)),
        )
        .ok();
    }

    pub fn escalation(&mut self, message: &str) {
        execute!(
            self.stdout,
            Print("\r\n  "),
            SetForegroundColor(Color::Red),
            SetAttribute(Attribute::Bold),
            Print("▲ escalation"),
            SetAttribute(Attribute::Reset),
            ResetColor,
            Print(format!(": {}\r\n", message)),
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
            Print("\r\n  "),
            SetForegroundColor(Color::Green),
            Print(format!(
                "all {} tasks complete in {}s",
                total_tasks, total_duration_secs
            )),
            ResetColor,
            Print("\r\n"),
        )
        .ok();
    }

    pub fn task_failure(&mut self, task_id: &str, error: &str) {
        execute!(
            self.stdout,
            Print("\r\n  "),
            SetForegroundColor(Color::Red),
            Print(format!("task {} failed: {}", task_id, error)),
            ResetColor,
            Print("\r\n"),
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
            Print("\r\n  "),
            SetForegroundColor(color),
            Print(format!(
                "{} validation {}",
                icon,
                if passed { "passed" } else { "failed" }
            )),
            ResetColor,
            Print(format!(
                "  blocking: {}  minor: {}  tests: {} passed, {} failed\r\n",
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
            Print(format!("{}\r\n", message)),
        )
        .ok();
    }

    pub fn run_metrics_summary(&mut self, lines: &[String]) {
        execute!(
            self.stdout,
            Print("\r\n"),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
        )
        .ok();
        for line in lines {
            execute!(self.stdout, Print(format!("  {}\r\n", line)),).ok();
        }
        execute!(self.stdout, ResetColor,).ok();
    }

    pub fn cycling_detected(&mut self, context: &str) {
        execute!(
            self.stdout,
            Print("\r\n  "),
            SetForegroundColor(Color::Yellow),
            Print("■ cycling detected"),
            ResetColor,
            Print(format!(": {} — breaking loop early\r\n", context)),
        )
        .ok();
    }

    pub fn run_complete(&mut self, run_id: &str) {
        execute!(
            self.stdout,
            Print("\r\n"),
            SetForegroundColor(Color::Green),
            SetAttribute(Attribute::Bold),
            Print("  ✓ run complete"),
            SetAttribute(Attribute::Reset),
            ResetColor,
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(format!("  ({})", run_id)),
            ResetColor,
            Print("\r\n"),
        )
        .ok();
    }

    /// Show a blocking screen when no AI CLI tools are installed.
    /// Waits for Esc or any key to exit.
    pub fn no_providers_screen(&mut self) {
        use crossterm::event::{self, Event, KeyCode, KeyEvent};

        let dim = Color::Rgb {
            r: 130,
            g: 130,
            b: 130,
        };
        let yellow = Color::Yellow;
        let kora_purple = Color::Rgb {
            r: 108,
            g: 92,
            b: 231,
        };

        execute!(
            self.stdout,
            Print("\r\n"),
            SetForegroundColor(yellow),
            Print("  ⚠ No AI CLI tools detected.\r\n"),
            ResetColor,
            Print("\r\n"),
            SetForegroundColor(dim),
            Print("  Kora needs at least one AI coding agent installed:\r\n"),
            Print("\r\n"),
            Print("    • "),
            SetForegroundColor(Color::Cyan),
            Print("claude"),
            SetForegroundColor(dim),
            Print("   https://docs.anthropic.com/en/docs/claude-code\r\n"),
            Print("    • "),
            SetForegroundColor(Color::Cyan),
            Print("codex"),
            SetForegroundColor(dim),
            Print("    https://github.com/openai/codex\r\n"),
            Print("    • "),
            SetForegroundColor(Color::Cyan),
            Print("gemini"),
            SetForegroundColor(dim),
            Print("   https://github.com/google-gemini/gemini-cli\r\n"),
            Print("\r\n"),
            Print("  Install one of the above, then restart kora.\r\n"),
            Print("\r\n"),
            SetForegroundColor(kora_purple),
            Print("  Esc"),
            SetForegroundColor(dim),
            Print(" to exit"),
            ResetColor,
        )
        .ok();
        self.stdout.flush().ok();

        // Wait for Esc or any key
        crossterm::terminal::enable_raw_mode().ok();
        loop {
            if let Ok(Event::Key(KeyEvent { code, .. })) = event::read() {
                match code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => break,
                    _ => break,
                }
            }
        }
        crossterm::terminal::disable_raw_mode().ok();
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}
