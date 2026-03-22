use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

use crate::state::RunState;
use crate::terminal::Renderer;

#[derive(Debug, PartialEq)]
pub enum MetaCommand {
    Status,
    Configure,
    Help,
    Clear,
    Quit,
    None(String),
}

pub fn parse_meta_command(input: &str) -> MetaCommand {
    let trimmed = input.trim();
    match trimmed {
        "/status" => MetaCommand::Status,
        "/config" | "/configure" => MetaCommand::Configure,
        "/help" => MetaCommand::Help,
        "/clear" | "clear" => MetaCommand::Clear,
        "/exit" | "/quit" | "exit" | "quit" => MetaCommand::Quit,
        _ => MetaCommand::None(trimmed.to_string()),
    }
}

pub fn handle_status(_renderer: &mut Renderer, last_run: Option<&RunState>) {
    let _ = show_status_panel(last_run);
}

fn show_status_panel(last_run: Option<&RunState>) -> io::Result<()> {
    let mut stdout = io::stdout();

    let lines: Vec<String> = match last_run {
        Some(run) => {
            let status_label = run.status.label();
            let request_display = truncate_request(&run.request, 50);
            vec![
                String::new(),
                "  Last Run".to_string(),
                "  ─────────────────────────────────────────".to_string(),
                String::new(),
                format!("  Request    \"{}\"", request_display),
                format!("  Status     {}", status_label),
                format!("  Run ID     {}", run.id),
                String::new(),
            ]
        }
        None => {
            vec![
                String::new(),
                "  Status".to_string(),
                "  ─────────────────────────────────────────".to_string(),
                String::new(),
                "  No runs in this session yet.".to_string(),
                String::new(),
            ]
        }
    };

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    for line in &lines {
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Print(format!("{}\r\n", line))
        )?;
    }

    execute!(
        stdout,
        cursor::MoveToColumn(0),
        SetForegroundColor(Color::Rgb {
            r: 130,
            g: 130,
            b: 130
        }),
        Print("  Esc to close"),
        ResetColor,
    )?;
    stdout.flush()?;

    loop {
        if let Event::Key(KeyEvent { code: _, .. }) = event::read()? {
            break;
        }
    }

    let total = lines.len();
    execute!(
        stdout,
        cursor::MoveUp(total as u16),
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
    )?;

    execute!(stdout, cursor::Show)?;
    terminal::disable_raw_mode()?;

    Ok(())
}

pub fn handle_help(_renderer: &mut Renderer) {
    let _ = show_help_panel();
}

fn show_help_panel() -> io::Result<()> {
    let mut stdout = io::stdout();

    let lines = [
        "",
        "  Commands",
        "  ─────────────────────────────────────────",
        "",
        "  /configure   Edit configuration",
        "  /status      Current/recent run info",
        "  /clear       Reset session",
        "  /help        This help panel",
        "  /exit        Exit kora",
        "",
        "  Or type a request to start a new run.",
        "",
    ];

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    for line in &lines {
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Print(format!("{}\r\n", line))
        )?;
    }

    execute!(
        stdout,
        cursor::MoveToColumn(0),
        SetForegroundColor(Color::Rgb {
            r: 130,
            g: 130,
            b: 130
        }),
        Print("  Esc to close"),
        ResetColor,
    )?;
    stdout.flush()?;

    // Wait for any key to dismiss
    loop {
        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => break,
                _ => break,
            }
        }
    }

    // Clean up: lines + hint
    let total = lines.len();
    execute!(
        stdout,
        cursor::MoveUp(total as u16),
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
    )?;

    execute!(stdout, cursor::Show)?;
    terminal::disable_raw_mode()?;

    Ok(())
}

fn truncate_request(request: &str, max_len: usize) -> String {
    let first_line = request.lines().next().unwrap_or(request);
    if first_line.len() > max_len {
        format!("{}...", &first_line[..max_len])
    } else {
        first_line.to_string()
    }
}
