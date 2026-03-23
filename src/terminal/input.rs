use crossterm::{
    cursor,
    event::{
        self, Event, KeyCode, KeyEvent, KeyModifiers,
        KeyboardEnhancementFlags, PushKeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    },
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

// Kora brand purple
const KORA_PURPLE: Color = Color::Rgb { r: 108, g: 92, b: 231 };
const DIM: Color = Color::Rgb { r: 130, g: 130, b: 130 };

/// Status bar info for the prompt area.
pub struct PromptStatus {
    pub preset: String,
    pub branch: String,
    pub checkpoints: usize,
}

fn box_width() -> usize {
    terminal::size()
        .map(|(w, _)| (w as usize).saturating_sub(10))
        .unwrap_or(70)
}

/// Draw the prompt area and read multiline input.
pub fn read_user_input(status: &PromptStatus) -> io::Result<String> {
    let mut stdout = io::stdout();

    let checkpoint_str = if status.checkpoints == 0 {
        "no checkpoints".to_string()
    } else {
        format!("{} checkpoint{}", status.checkpoints, if status.checkpoints == 1 { "" } else { "s" })
    };

    let status_line = format!(
        "  \x1b[38;2;130;130;130mPreset: \x1b[36m{}\x1b[38;2;130;130;130m · Branch: \x1b[32m{}\x1b[38;2;130;130;130m · \x1b[33m{}\x1b[0m",
        status.preset, status.branch, checkpoint_str
    );

    // Draw initial box (1 line of input)
    let mut lines: Vec<String> = vec![String::new()];
    let mut cur_line: usize = 0;
    let mut cur_col: usize = 0;

    draw_box(&mut stdout, &lines, cur_line, cur_col)?;

    // Status bar
    execute!(
        stdout,
        Print("  "),
        SetForegroundColor(DIM), Print("Preset: "),
        SetForegroundColor(Color::Cyan), Print(&status.preset),
        SetForegroundColor(DIM), Print(" · Branch: "),
        SetForegroundColor(Color::Green), Print(&status.branch),
        SetForegroundColor(DIM), Print(" · "),
        SetForegroundColor(Color::Yellow), Print(&checkpoint_str),
        ResetColor,
    )?;

    // Move cursor up to input line: up 2 (status bar + bottom border)
    execute!(stdout, cursor::MoveUp(2), cursor::MoveToColumn(PROMPT_COL))?;
    stdout.flush()?;

    terminal::enable_raw_mode()?;

    // Enable enhanced keyboard protocol so Shift+Enter is distinguishable from Enter
    // Silently ignore if terminal doesn't support it (falls back to Alt+Enter / Ctrl+J)
    execute!(
        stdout,
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
    ).ok();

    // Now in raw mode — cursor::position() works. Record the top border row.
    let (_, input_row) = cursor::position()?;
    let mut top_border_row = input_row.saturating_sub(1); // input line is 1 below top border

    let mut dd_sel: usize = 0;
    let mut dd_visible = false;

    loop {
        if let Event::Key(KeyEvent { code, modifiers, .. }) = event::read()? {
            match code {
                // Ctrl+J: reliable newline (Shift+Enter and Alt+Enter also work if terminal supports)
                KeyCode::Char('j') if modifiers.contains(KeyModifiers::CONTROL) => {
                    let rest = lines[cur_line][cur_col..].to_string();
                    lines[cur_line].truncate(cur_col);
                    cur_line += 1;
                    lines.insert(cur_line, rest);
                    cur_col = 0;
                    redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                }
                // Shift+Enter or Alt+Enter: new line (terminal-dependent)
                KeyCode::Enter if modifiers.contains(KeyModifiers::SHIFT) || modifiers.contains(KeyModifiers::ALT) => {
                    let rest = lines[cur_line][cur_col..].to_string();
                    lines[cur_line].truncate(cur_col);
                    cur_line += 1;
                    lines.insert(cur_line, rest);
                    cur_col = 0;
                    redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                }
                // Enter: submit (or autocomplete if dropdown visible)
                KeyCode::Enter => {
                    if dd_visible {
                        let input_text = lines[0].clone();
                        let m = get_matches(&input_text);
                        if dd_sel < m.len() {
                            lines[0] = m[dd_sel].0.to_string();
                            cur_col = lines[0].len();
                            redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                        }
                        hide_dropdown(&mut stdout, &lines, &status_line)?;
                    }
                    // Clear the entire box before returning
                    execute!(stdout, cursor::MoveTo(0, top_border_row), Clear(ClearType::FromCursorDown))?;
                    execute!(stdout, PopKeyboardEnhancementFlags).ok();
                    terminal::disable_raw_mode()?;
                    return Ok(lines.join("\n").trim().to_string());
                }
                KeyCode::Tab if dd_visible => {
                    let input_text = lines[0].clone();
                    let m = get_matches(&input_text);
                    if dd_sel < m.len() {
                        lines[0] = m[dd_sel].0.to_string();
                        cur_col = lines[0].len();
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                        hide_dropdown(&mut stdout, &lines, &status_line)?;
                        dd_visible = false;
                    }
                }
                KeyCode::Up if dd_visible => {
                    dd_sel = dd_sel.saturating_sub(1);
                    show_dropdown(&mut stdout, &lines, &lines[0], dd_sel)?;
                }
                KeyCode::Down if dd_visible => {
                    let m = get_matches(&lines[0]);
                    if dd_sel + 1 < m.len() { dd_sel += 1; }
                    show_dropdown(&mut stdout, &lines, &lines[0], dd_sel)?;
                }
                KeyCode::Up if !dd_visible && cur_line > 0 => {
                    cur_line -= 1;
                    cur_col = cur_col.min(lines[cur_line].len());
                    redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                    stdout.flush()?;
                }
                KeyCode::Down if !dd_visible && cur_line + 1 < lines.len() => {
                    cur_line += 1;
                    cur_col = cur_col.min(lines[cur_line].len());
                    redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                    stdout.flush()?;
                }
                KeyCode::Esc => {
                    if dd_visible {
                        hide_dropdown(&mut stdout, &lines, &status_line)?;
                        dd_visible = false;
                    } else {
                        lines.clear();
                        lines.push(String::new());
                        cur_line = 0;
                        cur_col = 0;
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                    }
                }
                KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                    if dd_visible { hide_dropdown(&mut stdout, &lines, &status_line)?; }
                    execute!(stdout, PopKeyboardEnhancementFlags).ok();
                    terminal::disable_raw_mode()?;
                    let lines_below = (lines.len() - 1 - cur_line) as u16 + 2;
                    execute!(stdout, cursor::MoveDown(lines_below), Print("\r\n"))?;
                    return Ok(String::new());
                }
                KeyCode::Char('d') if modifiers.contains(KeyModifiers::CONTROL) => {
                    if dd_visible { hide_dropdown(&mut stdout, &lines, &status_line)?; }
                    execute!(stdout, PopKeyboardEnhancementFlags).ok();
                    terminal::disable_raw_mode()?;
                    let lines_below = (lines.len() - 1 - cur_line) as u16 + 2;
                    execute!(stdout, cursor::MoveDown(lines_below), Print("\r\n"))?;
                    return Ok(String::new());
                }
                KeyCode::Char(c) if modifiers.is_empty() || modifiers == KeyModifiers::SHIFT => {
                    lines[cur_line].insert(cur_col, c);
                    cur_col += 1;
                    redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                    if lines.len() == 1 {
                        dd_sel = 0;
                        refresh_dropdown(&mut stdout, &lines, &lines[0], dd_sel, &mut dd_visible, &status_line)?;
                    }
                }
                // Alt+Backspace: delete word
                KeyCode::Backspace if modifiers.contains(KeyModifiers::ALT) => {
                    if cur_col > 0 {
                        // Delete back to previous word boundary (or start of line)
                        let line = &lines[cur_line];
                        let mut new_col = cur_col;
                        // Skip trailing spaces
                        while new_col > 0 && line.as_bytes()[new_col - 1] == b' ' {
                            new_col -= 1;
                        }
                        // Skip word chars
                        while new_col > 0 && line.as_bytes()[new_col - 1] != b' ' {
                            new_col -= 1;
                        }
                        lines[cur_line].drain(new_col..cur_col);
                        cur_col = new_col;
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                        if lines.len() == 1 {
                            dd_sel = 0;
                            refresh_dropdown(&mut stdout, &lines, &lines[0], dd_sel, &mut dd_visible, &status_line)?;
                        }
                    }
                }
                KeyCode::Backspace => {
                    if cur_col > 0 {
                        cur_col -= 1;
                        lines[cur_line].remove(cur_col);
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                        if lines.len() == 1 {
                            dd_sel = 0;
                            refresh_dropdown(&mut stdout, &lines, &lines[0], dd_sel, &mut dd_visible, &status_line)?;
                        }
                    } else if cur_line > 0 {
                        // Merge with previous line — cursor was on cur_line before decrement
                        let current = lines.remove(cur_line);
                        cur_line -= 1;
                        cur_col = lines[cur_line].len();
                        lines[cur_line].push_str(&current);
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                    }
                }
                KeyCode::Delete => {
                    if cur_col < lines[cur_line].len() {
                        lines[cur_line].remove(cur_col);
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                    } else if cur_line + 1 < lines.len() {
                        // Merge with next line — cursor stays on cur_line
                        let next = lines.remove(cur_line + 1);
                        lines[cur_line].push_str(&next);
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                    }
                }
                KeyCode::Left if !dd_visible => {
                    if cur_col > 0 {
                        cur_col -= 1;
                        execute!(stdout, cursor::MoveLeft(1))?;
                        stdout.flush()?;
                    } else if cur_line > 0 {
                        cur_line -= 1;
                        cur_col = lines[cur_line].len();
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                        stdout.flush()?;
                    }
                }
                KeyCode::Right if !dd_visible => {
                    if cur_col < lines[cur_line].len() {
                        cur_col += 1;
                        execute!(stdout, cursor::MoveRight(1))?;
                        stdout.flush()?;
                    } else if cur_line + 1 < lines.len() {
                        cur_line += 1;
                        cur_col = 0;
                        redraw_from_top(&mut stdout, &lines, cur_line, cur_col, &mut top_border_row, &status_line)?;
                        stdout.flush()?;
                    }
                }
                KeyCode::Home => {
                    cur_col = 0;
                    execute!(stdout, cursor::MoveToColumn(PROMPT_COL))?;
                    stdout.flush()?;
                }
                KeyCode::End => {
                    cur_col = lines[cur_line].len();
                    execute!(stdout, cursor::MoveToColumn(PROMPT_COL + cur_col as u16))?;
                    stdout.flush()?;
                }
                _ => {}
            }
        }
    }
}

const PROMPT_COL: u16 = 4; // after "  > " or "    "

/// How many chars fit on one visual line after the prompt.
/// Must align with box_width so text doesn't overflow past the border.
fn text_width() -> usize {
    // Border line = "  +" + box_width + "+" = PROMPT_COL-2 + box_width + 1 chars
    // Text line = PROMPT_COL chars prefix + text
    // Text must not exceed box_width - 2 (for the "  " or "> " prefix within the border area)
    box_width().saturating_sub(2)
}

/// Print a logical line with manual wrapping — continuation lines get indented to PROMPT_COL.
fn print_wrapped_line(stdout: &mut io::Stdout, line: &str, is_first: bool) -> io::Result<usize> {
    let tw = text_width();
    let prefix = if is_first { "> " } else { "  " };
    let indent = " ".repeat(PROMPT_COL as usize);

    if line.len() <= tw {
        // Fits on one line
        execute!(stdout, SetForegroundColor(KORA_PURPLE), Print("  "), ResetColor)?;
        execute!(stdout, Print(prefix), Print(line), Print("\r\n"))?;
        return Ok(1);
    }

    // First visual line
    let first_chunk = &line[..tw];
    execute!(stdout, SetForegroundColor(KORA_PURPLE), Print("  "), ResetColor)?;
    execute!(stdout, Print(prefix), Print(first_chunk), Print("\r\n"))?;

    // Remaining chunks with indent
    let mut pos = tw;
    let mut visual_lines = 1usize;
    while pos < line.len() {
        let end = (pos + tw).min(line.len());
        let chunk = &line[pos..end];
        execute!(stdout, Print(&indent), Print(chunk), Print("\r\n"))?;
        pos = end;
        visual_lines += 1;
    }

    Ok(visual_lines)
}

/// Count visual lines a logical line would take.
fn visual_line_count(line: &str) -> usize {
    let tw = text_width();
    if tw == 0 { return 1; }
    if line.len() <= tw { 1 } else { 1 + (line.len() - tw + tw - 1) / tw }
}

/// Draw the full box with all input lines.
fn draw_box(stdout: &mut io::Stdout, lines: &[String], _cur_line: usize, _cur_col: usize) -> io::Result<()> {
    let bw = box_width();
    let bar = "─".repeat(bw);
    let top = format!("  +{}+", bar);
    let bottom = format!("  +{}+", bar);

    execute!(stdout, SetForegroundColor(KORA_PURPLE), Print(&top), Print("\r\n"), ResetColor)?;

    for (i, line) in lines.iter().enumerate() {
        print_wrapped_line(stdout, line, i == 0)?;
    }

    execute!(stdout, SetForegroundColor(KORA_PURPLE), Print(&bottom), Print("\r\n"), ResetColor)?;
    Ok(())
}

/// Redraw everything using absolute row positioning. Handles wrapped lines correctly.
fn redraw_from_top(
    stdout: &mut io::Stdout,
    lines: &[String],
    cur_line: usize,
    cur_col: usize,
    top_border_row: &mut u16,
    status_line: &str,
) -> io::Result<()> {
    let tw = text_width();

    // Move to stored top border row, clear everything below
    execute!(stdout, cursor::MoveTo(0, *top_border_row), Clear(ClearType::FromCursorDown))?;
    draw_box(stdout, lines, cur_line, cur_col)?;
    execute!(stdout, Print(status_line))?;

    // Detect if terminal scrolled: check where cursor is after drawing
    let (_, after_draw_row) = cursor::position()?;
    // The status bar should be at: top_border + 1 (top) + visual_rows + 1 (bottom) = expected row
    let total_visual: u16 = lines.iter().map(|l| visual_line_count(l) as u16).sum();
    let expected_status_row = *top_border_row + 1 + total_visual + 1;
    // If actual row < expected, terminal scrolled
    if after_draw_row < expected_status_row {
        let scroll_amount = expected_status_row - after_draw_row;
        *top_border_row = top_border_row.saturating_sub(scroll_amount);
    }

    // Position cursor using updated top_border_row
    let mut target_row = *top_border_row + 1; // after top border
    for i in 0..cur_line {
        target_row += visual_line_count(&lines[i]) as u16;
    }
    let (target_vrow, target_col) = if tw > 0 && cur_col > tw {
        let row = (cur_col - tw) / tw + 1;
        let col = PROMPT_COL as usize + (cur_col - tw) % tw;
        (row as u16, col as u16)
    } else {
        (0, PROMPT_COL + cur_col as u16)
    };
    execute!(stdout, cursor::MoveTo(target_col, target_row + target_vrow))?;

    stdout.flush()
}

/// Fast-path redraw for single-char edits. Uses manual wrapping and cursor::position().
fn redraw_from_line(
    stdout: &mut io::Stdout,
    lines: &[String],
    cur_line: usize,
    cur_col: usize,
    status_line: &str,
) -> io::Result<()> {
    let bw = box_width();
    let bar = "─".repeat(bw);
    let tw = text_width();

    // Get current cursor position — figure out the first visual row of current line
    let (_, cur_row) = cursor::position()?;
    // How many visual rows down from the first row is the cursor?
    let cur_visual_row_in_line = if tw > 0 && cur_col > tw { (cur_col - tw) / tw + 1 } else { 0 };
    let first_row = cur_row.saturating_sub(cur_visual_row_in_line as u16);

    // Clear from first row of current line down
    execute!(stdout, cursor::MoveTo(0, first_row), Clear(ClearType::FromCursorDown))?;

    // Reprint current line and all remaining lines with manual wrapping
    let mut visual_rows_from_first = 0u16;
    for (i, line) in lines[cur_line..].iter().enumerate() {
        let line_idx = cur_line + i;
        let vl = print_wrapped_line(stdout, line, line_idx == 0)?;
        if i == 0 {
            // Track visual rows for cursor positioning
            visual_rows_from_first = 0; // we'll calculate target separately
        }
        let _ = vl;
    }

    // Bottom border + status
    let bottom = format!("  +{}+", bar);
    execute!(stdout, SetForegroundColor(KORA_PURPLE), Print(&bottom), Print("\r\n"), ResetColor)?;
    execute!(stdout, Print(status_line))?;

    // Position cursor: which visual row and col for cur_col?
    let (target_visual_row, target_col) = if tw > 0 && cur_col > tw {
        let row = (cur_col - tw) / tw + 1;
        let col = PROMPT_COL as usize + (cur_col - tw) % tw; // continuation lines start at PROMPT_COL
        (row as u16, col as u16)
    } else {
        (0, PROMPT_COL + cur_col as u16)
    };

    execute!(stdout, cursor::MoveTo(target_col, first_row + target_visual_row))?;
    stdout.flush()
}

// ── Dropdown (only for single-line input starting with /) ───────────

fn get_matches(input: &str) -> Vec<(&'static str, &'static str)> {
    if !input.starts_with('/') || input.contains(' ') { return vec![]; }
    COMMANDS.iter().filter(|(cmd, _)| cmd.starts_with(input)).copied().collect()
}

fn refresh_dropdown(
    stdout: &mut io::Stdout,
    lines: &[String],
    input: &str,
    sel: usize,
    visible: &mut bool,
    status_line: &str,
) -> io::Result<()> {
    let m = get_matches(input);
    if !m.is_empty() {
        show_dropdown(stdout, lines, input, sel)?;
        *visible = true;
    } else if *visible {
        hide_dropdown(stdout, lines, status_line)?;
        *visible = false;
    }
    Ok(())
}

/// Lines below cursor to the status bar area.
fn lines_below_cursor(lines: &[String], cur_line: usize) -> u16 {
    // From current line: remaining input lines + bottom border + status bar (but status is replaced)
    (lines.len() - 1 - cur_line) as u16 + 2
}

fn show_dropdown(stdout: &mut io::Stdout, lines: &[String], input: &str, sel: usize) -> io::Result<()> {
    let m = get_matches(input);
    if m.is_empty() { return Ok(()); }

    let below = lines_below_cursor(lines, 0); // dropdown only shows for line 0
    execute!(stdout, cursor::MoveDown(below), cursor::MoveToColumn(0))?;
    execute!(stdout, Clear(ClearType::FromCursorDown))?;

    for (i, (cmd, desc)) in m.iter().enumerate() {
        if i > 0 { execute!(stdout, Print("\r\n"), cursor::MoveToColumn(0))?; }
        if i == sel {
            execute!(stdout,
                Print("  "), SetForegroundColor(KORA_PURPLE), Print("▸ "),
                SetForegroundColor(Color::White), Print(format!("{:<16}", cmd)),
                SetForegroundColor(DIM), Print(desc), ResetColor,
            )?;
        } else {
            execute!(stdout,
                SetForegroundColor(DIM), Print(format!("    {:<16}{}", cmd, desc)), ResetColor,
            )?;
        }
    }

    let up = below as usize + m.len() - 1;
    let input_col = PROMPT_COL + input.len() as u16;
    execute!(stdout, cursor::MoveUp(up as u16), cursor::MoveToColumn(input_col))?;
    stdout.flush()
}

fn hide_dropdown(stdout: &mut io::Stdout, lines: &[String], status_line: &str) -> io::Result<()> {
    let (col, _) = cursor::position()?;
    let below = lines_below_cursor(lines, 0);
    execute!(stdout, cursor::MoveDown(below), cursor::MoveToColumn(0))?;
    execute!(stdout, Clear(ClearType::FromCursorDown))?;
    execute!(stdout, Print(status_line))?;
    execute!(stdout, cursor::MoveUp(below), cursor::MoveToColumn(col))?;
    stdout.flush()
}

/// Total lines the prompt box occupies (for clear_last_input).
pub fn prompt_lines(num_input_lines: usize) -> u16 {
    // top border + input lines + bottom border + status bar
    (1 + num_input_lines + 1 + 1) as u16
}

/// Erase the last prompt area.
pub fn clear_last_input() -> io::Result<()> {
    let mut stdout = io::stdout();
    // Assume single-line input was submitted (most common case)
    let lines = prompt_lines(1);
    execute!(stdout, cursor::MoveUp(lines), cursor::MoveToColumn(0), Clear(ClearType::FromCursorDown))?;
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
