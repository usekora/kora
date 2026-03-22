use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

/// Available commands with descriptions for autocomplete.
const COMMANDS: &[(&str, &str)] = &[
    ("/configure", "Edit configuration"),
    ("/status", "Current/recent run info"),
    ("/clear", "Reset session"),
    ("/help", "This help panel"),
    ("/exit", "Exit kora"),
];

/// Lines below cursor to the status bar (bottom border + status bar).
const LINES_BELOW_CURSOR: u16 = 2;

// Kora brand purple
const KORA_PURPLE: Color = Color::Rgb {
    r: 108,
    g: 92,
    b: 231,
};
const DIM: Color = Color::Rgb {
    r: 130,
    g: 130,
    b: 130,
};

fn read_line_inner(
    stdout: &mut io::Stdout,
    input: &mut String,
    cursor_pos: &mut usize,
    prompt_col: u16,
    status_line: &str,
) -> io::Result<()> {
    let mut dd_sel: usize = 0;
    let mut dd_visible = false;

    loop {
        if let Event::Key(KeyEvent {
            code, modifiers, ..
        }) = event::read()?
        {
            match code {
                KeyCode::Enter => {
                    if dd_visible {
                        let m = get_matches(input);
                        if dd_sel < m.len() {
                            *input = m[dd_sel].0.to_string();
                            *cursor_pos = input.len();
                            redraw_input(stdout, input, *cursor_pos, prompt_col)?;
                        }
                        hide_dropdown(stdout, status_line)?;
                    }
                    return Ok(());
                }
                KeyCode::Tab if dd_visible => {
                    let m = get_matches(input);
                    if dd_sel < m.len() {
                        *input = m[dd_sel].0.to_string();
                        *cursor_pos = input.len();
                        redraw_input(stdout, input, *cursor_pos, prompt_col)?;
                        hide_dropdown(stdout, status_line)?;
                        dd_visible = false;
                    }
                }
                KeyCode::Up if dd_visible => {
                    dd_sel = dd_sel.saturating_sub(1);
                    show_dropdown(stdout, input, dd_sel)?;
                }
                KeyCode::Down if dd_visible => {
                    let m = get_matches(input);
                    if dd_sel + 1 < m.len() {
                        dd_sel += 1;
                    }
                    show_dropdown(stdout, input, dd_sel)?;
                }
                KeyCode::Esc => {
                    if dd_visible {
                        hide_dropdown(stdout, status_line)?;
                        dd_visible = false;
                    } else {
                        input.clear();
                        *cursor_pos = 0;
                        redraw_input(stdout, input, *cursor_pos, prompt_col)?;
                    }
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    if dd_visible {
                        hide_dropdown(stdout, status_line)?;
                    }
                    input.clear();
                    return Ok(());
                }
                KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                    if dd_visible {
                        hide_dropdown(stdout, status_line)?;
                    }
                    input.clear();
                    return Ok(());
                }
                KeyCode::Char(c) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
                    input.insert(*cursor_pos, c);
                    *cursor_pos += 1;
                    redraw_input(stdout, input, *cursor_pos, prompt_col)?;
                    dd_sel = 0;
                    refresh_dropdown(stdout, input, dd_sel, &mut dd_visible, status_line)?;
                }
                KeyCode::Backspace => {
                    if *cursor_pos > 0 {
                        *cursor_pos -= 1;
                        input.remove(*cursor_pos);
                        redraw_input(stdout, input, *cursor_pos, prompt_col)?;
                        dd_sel = 0;
                        refresh_dropdown(stdout, input, dd_sel, &mut dd_visible, status_line)?;
                    }
                }
                KeyCode::Delete => {
                    if *cursor_pos < input.len() {
                        input.remove(*cursor_pos);
                        redraw_input(stdout, input, *cursor_pos, prompt_col)?;
                        dd_sel = 0;
                        refresh_dropdown(stdout, input, dd_sel, &mut dd_visible, status_line)?;
                    }
                }
                KeyCode::Left if !dd_visible => {
                    if *cursor_pos > 0 {
                        *cursor_pos -= 1;
                        execute!(*stdout, cursor::MoveLeft(1))?;
                        stdout.flush()?;
                    }
                }
                KeyCode::Right if !dd_visible => {
                    if *cursor_pos < input.len() {
                        *cursor_pos += 1;
                        execute!(*stdout, cursor::MoveRight(1))?;
                        stdout.flush()?;
                    }
                }
                KeyCode::Home => {
                    *cursor_pos = 0;
                    execute!(*stdout, cursor::MoveToColumn(prompt_col))?;
                    stdout.flush()?;
                }
                KeyCode::End => {
                    *cursor_pos = input.len();
                    execute!(
                        *stdout,
                        cursor::MoveToColumn(prompt_col + input.len() as u16)
                    )?;
                    stdout.flush()?;
                }
                _ => {}
            }
        }
    }
}

fn get_matches(input: &str) -> Vec<(&'static str, &'static str)> {
    if !input.starts_with('/') || input.contains(' ') {
        return vec![];
    }
    COMMANDS
        .iter()
        .filter(|(cmd, _)| cmd.starts_with(input))
        .copied()
        .collect()
}

/// Show or hide dropdown based on current input.
fn refresh_dropdown(
    stdout: &mut io::Stdout,
    input: &str,
    sel: usize,
    visible: &mut bool,
    status_line: &str,
) -> io::Result<()> {
    let m = get_matches(input);
    if !m.is_empty() {
        show_dropdown(stdout, input, sel)?;
        *visible = true;
    } else if *visible {
        hide_dropdown(stdout, status_line)?;
        *visible = false;
    }
    Ok(())
}

/// Draw dropdown replacing the status bar. Uses MoveDown/MoveUp (no SavePosition).
fn show_dropdown(stdout: &mut io::Stdout, input: &str, sel: usize) -> io::Result<()> {
    let m = get_matches(input);
    if m.is_empty() {
        return Ok(());
    }

    // Move from input line down past bottom border to status bar area
    execute!(
        *stdout,
        cursor::MoveDown(LINES_BELOW_CURSOR),
        cursor::MoveToColumn(0)
    )?;
    execute!(*stdout, Clear(ClearType::FromCursorDown))?;

    for (i, (cmd, desc)) in m.iter().enumerate() {
        if i > 0 {
            execute!(*stdout, Print("\r\n"), cursor::MoveToColumn(0))?;
        }
        if i == sel {
            execute!(
                *stdout,
                Print("  "),
                SetForegroundColor(KORA_PURPLE),
                Print("▸ "),
                SetForegroundColor(Color::White),
                Print(format!("{:<16}", cmd)),
                SetForegroundColor(DIM),
                Print(desc),
                ResetColor,
            )?;
        } else {
            execute!(
                *stdout,
                SetForegroundColor(DIM),
                Print(format!("    {:<16}{}", cmd, desc)),
                ResetColor,
            )?;
        }
    }

    // Move back up and restore column to cursor position in input
    let up = LINES_BELOW_CURSOR as usize + m.len() - 1;
    let input_col = 6 + input.len() as u16; // prompt_col + input length
    execute!(
        *stdout,
        cursor::MoveUp(up as u16),
        cursor::MoveToColumn(input_col)
    )?;
    stdout.flush()
}

/// Clear dropdown and restore status bar.
fn hide_dropdown(stdout: &mut io::Stdout, status_line: &str) -> io::Result<()> {
    // Get current column so we can restore it
    let (col, _) = cursor::position()?;
    execute!(
        *stdout,
        cursor::MoveDown(LINES_BELOW_CURSOR),
        cursor::MoveToColumn(0)
    )?;
    execute!(*stdout, Clear(ClearType::FromCursorDown))?;
    execute!(*stdout, Print(status_line))?;
    execute!(
        *stdout,
        cursor::MoveUp(LINES_BELOW_CURSOR),
        cursor::MoveToColumn(col)
    )?;
    stdout.flush()
}

fn redraw_input(
    stdout: &mut io::Stdout,
    input: &str,
    cursor_pos: usize,
    prompt_col: u16,
) -> io::Result<()> {
    execute!(
        *stdout,
        cursor::MoveToColumn(prompt_col),
        Clear(ClearType::UntilNewLine),
        Print(input),
        cursor::MoveToColumn(prompt_col + cursor_pos as u16),
    )?;
    stdout.flush()
}

/// Status bar info for the prompt area.
pub struct PromptStatus {
    pub preset: String,
    pub branch: String,
    pub checkpoints: usize,
}

fn box_width() -> usize {
    terminal::size()
        .map(|(w, _)| (w as usize).saturating_sub(4))
        .unwrap_or(76)
}

const PROMPT_LINES: u16 = 4;

/// Draw the prompt area: bordered input box with status bar below.
pub fn read_user_input(status: &PromptStatus) -> io::Result<String> {
    let mut stdout = io::stdout();

    let checkpoint_str = if status.checkpoints == 0 {
        "no checkpoints".to_string()
    } else {
        format!(
            "{} checkpoint{}",
            status.checkpoints,
            if status.checkpoints == 1 { "" } else { "s" }
        )
    };

    let bar = "─".repeat(box_width());

    // Build status line string for redrawing after dropdown hides
    let status_line = format!(
        "  \x1b[38;2;130;130;130mPreset: \x1b[38;2;0;255;255m{}\x1b[38;2;130;130;130m · Branch: \x1b[38;2;0;255;0m{}\x1b[38;2;130;130;130m · \x1b[38;2;255;255;0m{}\x1b[0m",
        status.preset, status.branch, checkpoint_str
    );

    // Line 1: top border
    execute!(
        stdout,
        SetForegroundColor(KORA_PURPLE),
        Print(format!("  ╭{}\r\n", bar)),
        ResetColor
    )?;
    // Line 2: input line
    execute!(
        stdout,
        SetForegroundColor(KORA_PURPLE),
        Print("  │ ❯ "),
        ResetColor,
        Print("\r\n")
    )?;
    // Line 3: bottom border
    execute!(
        stdout,
        SetForegroundColor(KORA_PURPLE),
        Print(format!("  ╰{}\r\n", bar)),
        ResetColor
    )?;
    // Line 4: status bar
    execute!(
        stdout,
        Print("  "),
        SetForegroundColor(DIM),
        Print("Preset: "),
        SetForegroundColor(Color::Cyan),
        Print(&status.preset),
        SetForegroundColor(DIM),
        Print(" · Branch: "),
        SetForegroundColor(Color::Green),
        Print(&status.branch),
        SetForegroundColor(DIM),
        Print(" · "),
        SetForegroundColor(Color::Yellow),
        Print(&checkpoint_str),
        ResetColor,
    )?;

    // Move cursor to input line
    execute!(stdout, cursor::MoveUp(2), cursor::MoveToColumn(6))?;
    stdout.flush()?;

    let mut input = String::new();
    let mut cursor_pos: usize = 0;
    let prompt_col: u16 = 6;

    terminal::enable_raw_mode()?;
    let result = read_line_inner(
        &mut stdout,
        &mut input,
        &mut cursor_pos,
        prompt_col,
        &status_line,
    );
    terminal::disable_raw_mode()?;

    result?;

    // Move past bottom border + status
    execute!(stdout, cursor::MoveDown(2), Print("\r\n"))?;

    Ok(input.trim().to_string())
}

/// Erase the last prompt area.
pub fn clear_last_input() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        cursor::MoveUp(PROMPT_LINES),
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown)
    )?;
    Ok(())
}

/// Get the current git branch name.
pub fn get_git_branch(project_root: &std::path::Path) -> String {
    std::process::Command::new("git")
        .current_dir(project_root)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "—".to_string())
}
