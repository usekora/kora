use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Attribute, Color, Print, ResetColor, SetAttribute, SetForegroundColor},
    terminal::{self, Clear, ClearType},
};
use std::io::{self, Write};

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

/// Single-choice selector styled to match the settings menu.
/// On Esc, returns the original `current` index (no change).
pub fn select(_prompt: &str, options: &[&str], current: usize) -> io::Result<usize> {
    let mut stdout = io::stdout();
    let mut selected = current.min(options.len().saturating_sub(1));

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    // Blank line + options + blank line + hint = options.len() + 2 lines with \r\n
    let move_up = options.len() + 2;

    loop {
        // Top padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        for (i, option) in options.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;
            if i == selected {
                execute!(
                    stdout,
                    Print("  "),
                    SetForegroundColor(KORA_PURPLE),
                    Print("▸ "),
                    SetForegroundColor(Color::White),
                    Print(option),
                    ResetColor,
                    Print("\r\n"),
                )?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(DIM),
                    Print(format!("    {}", option)),
                    ResetColor,
                    Print("\r\n"),
                )?;
            }
        }

        // Bottom padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        // Hint
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("  "),
            SetForegroundColor(KORA_PURPLE),
            Print("Enter"),
            SetForegroundColor(DIM),
            Print(" select · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Esc"),
            SetForegroundColor(DIM),
            Print(" cancel"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    if selected < options.len() - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Enter => {
                    // Clean up and return
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show,
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(selected);
                }
                KeyCode::Char('q') | KeyCode::Esc => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show,
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(current.min(options.len().saturating_sub(1)));
                }
                _ => {}
            }
        }

        execute!(stdout, cursor::MoveUp(move_up as u16))?;
    }
}

/// Multi-choice selector styled to match the settings menu.
/// On Esc, returns the original `preselected` indices (no change).
pub fn multi_select(
    _prompt: &str,
    options: &[&str],
    preselected: &[usize],
) -> io::Result<Vec<usize>> {
    let mut stdout = io::stdout();
    let mut selected = 0usize;
    let mut toggled = vec![false; options.len()];
    for &idx in preselected {
        if idx < toggled.len() {
            toggled[idx] = true;
        }
    }

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    // Blank + options + blank + hint
    let move_up = options.len() + 2;

    loop {
        // Top padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        for (i, option) in options.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;
            let check = if toggled[i] { "●" } else { "○" };
            if i == selected {
                let check_color = if toggled[i] { Color::Green } else { DIM };
                execute!(
                    stdout,
                    Print("  "),
                    SetForegroundColor(KORA_PURPLE),
                    Print("▸ "),
                    SetForegroundColor(check_color),
                    Print(format!("{} ", check)),
                    SetForegroundColor(Color::White),
                    Print(option),
                    ResetColor,
                    Print("\r\n"),
                )?;
            } else {
                let check_color = if toggled[i] { Color::Green } else { DIM };
                execute!(
                    stdout,
                    Print("    "),
                    SetForegroundColor(check_color),
                    Print(format!("{} ", check)),
                    SetForegroundColor(DIM),
                    Print(option),
                    ResetColor,
                    Print("\r\n"),
                )?;
            }
        }

        // Bottom padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        // Hint
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("  "),
            SetForegroundColor(KORA_PURPLE),
            Print("Space"),
            SetForegroundColor(DIM),
            Print(" toggle · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Enter"),
            SetForegroundColor(DIM),
            Print(" confirm · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Esc"),
            SetForegroundColor(DIM),
            Print(" cancel"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    selected = selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    if selected < options.len() - 1 {
                        selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    toggled[selected] = !toggled[selected];
                }
                KeyCode::Enter => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show,
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(toggled
                        .iter()
                        .enumerate()
                        .filter(|(_, t)| **t)
                        .map(|(i, _)| i)
                        .collect());
                }
                KeyCode::Esc => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show,
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(preselected.to_vec());
                }
                _ => {}
            }
        }

        execute!(stdout, cursor::MoveUp(move_up as u16))?;
    }
}

/// A setting entry for the settings menu.
pub struct Setting {
    pub label: String,
    pub value: String,
}

/// Settings menu result.
pub enum SettingsAction {
    /// User selected a setting to edit (index).
    Edit(usize),
    /// User chose to exit.
    Exit,
}

/// Flat settings menu matching Claude Code's config panel style.
/// Returns which setting the user wants to edit, or Exit on Enter/Esc.
pub fn settings_menu(settings: &[Setting]) -> io::Result<SettingsAction> {
    let mut stdout = io::stdout();
    let mut selected = 0usize;

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    let result = run_settings_loop(settings, &mut stdout, &mut selected);

    // Clean up: blank line + settings + blank line + hint
    clear_settings_area(&mut stdout, settings.len())?;

    execute!(stdout, cursor::Show)?;
    terminal::disable_raw_mode()?;

    result
}

/// Draw the fixed header. Called once by run_configure.
pub fn draw_settings_header() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        Print("\r\n"),
        SetForegroundColor(Color::White),
        Print("  Configuration\r\n"),
        SetForegroundColor(DIM),
        Print("  ─────────────────────────────────────────\r\n"),
        ResetColor,
    )?;
    Ok(())
}

/// Show a description header above a sub-menu.
/// Returns the number of lines drawn (for cleanup).
pub fn show_submenu_desc(title: &str, description: &str) -> io::Result<u16> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        Print("\r\n"),
        SetForegroundColor(Color::White),
        Print(format!("  {}\r\n", title)),
        SetForegroundColor(DIM),
        Print(format!("  {}\r\n", description)),
        ResetColor,
    )?;
    Ok(3) // blank + title + description
}

/// Clear a sub-menu description header.
pub fn clear_submenu_desc(lines: u16) -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        cursor::MoveUp(lines),
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
    )?;
    Ok(())
}

/// Clear the fixed header. Called once when exiting configure.
pub fn clear_settings_header() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        cursor::MoveUp(3),
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
    )?;
    Ok(())
}

fn clear_settings_area(stdout: &mut io::Stdout, num_settings: usize) -> io::Result<()> {
    // blank line + settings (N) + blank line = N+2 lines with \r\n, hint has none
    let move_back = num_settings + 2;
    execute!(
        stdout,
        cursor::MoveUp(move_back as u16),
        cursor::MoveToColumn(0),
        Clear(ClearType::FromCursorDown),
    )?;
    Ok(())
}

// Column where values start
const VALUE_COL: usize = 36;

fn run_settings_loop(
    settings: &[Setting],
    stdout: &mut io::Stdout,
    selected: &mut usize,
) -> io::Result<SettingsAction> {
    // blank line + N settings + blank line = N+2 lines with \r\n
    let move_up = settings.len() + 2;
    let last_idx = settings.len() - 1;

    loop {
        // Top padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        for (i, setting) in settings.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;

            let label_with_pad = if setting.label.len() + 6 < VALUE_COL {
                format!("{:<width$}", setting.label, width = VALUE_COL - 6)
            } else {
                setting.label.clone()
            };

            if i == *selected {
                // Selected: purple accent ▸, white label, bold purple value
                execute!(
                    stdout,
                    Print("  "),
                    SetForegroundColor(KORA_PURPLE),
                    Print("▸ "),
                    SetForegroundColor(Color::White),
                    Print(&label_with_pad),
                    SetForegroundColor(KORA_PURPLE),
                    SetAttribute(Attribute::Bold),
                    Print(&setting.value),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                    Print("\r\n"),
                )?;
            } else {
                // Unselected: all grey, dotted fill
                let dots = ".".repeat(VALUE_COL.saturating_sub(setting.label.len() + 6 + 1));
                execute!(
                    stdout,
                    SetForegroundColor(Color::Rgb {
                        r: 130,
                        g: 130,
                        b: 130
                    }),
                    Print(format!("    {} {}", setting.label, dots)),
                    SetForegroundColor(Color::White),
                    Print(format!(" {}", setting.value)),
                    ResetColor,
                    Print("\r\n"),
                )?;
            }
        }

        // Bottom padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        // Hint line
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print("  "),
            SetForegroundColor(KORA_PURPLE),
            Print("Space"),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(" change · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Enter"),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(" save · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Esc"),
            SetForegroundColor(Color::Rgb {
                r: 130,
                g: 130,
                b: 130
            }),
            Print(" cancel"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    if *selected < last_idx {
                        *selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    clear_settings_area(stdout, settings.len())?;
                    return Ok(SettingsAction::Edit(*selected));
                }
                KeyCode::Enter => {
                    return Ok(SettingsAction::Exit);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    return Ok(SettingsAction::Exit);
                }
                _ => {}
            }
        }

        execute!(stdout, cursor::MoveUp(move_up as u16))?;
    }
}

/// A list where Space cycles the value for the selected item inline.
/// Returns true on Enter (confirmed), false on Esc (reverted).
/// The `available` slice marks which choices can be selected; unavailable ones
/// are skipped when cycling and shown with " (not installed)" in dim text.
pub fn toggle_list(
    labels: &[&str],
    values: &mut Vec<usize>,
    choices: &[&str],
    available: &[bool],
) -> io::Result<bool> {
    let mut stdout = io::stdout();
    let mut selected = 0usize;
    let original = values.clone();

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    let move_up = labels.len() + 2;
    let last_idx = labels.len() - 1;

    loop {
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        for (i, label) in labels.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;

            let choice_idx = values[i];
            let value = choices[choice_idx];
            let is_available = available.get(choice_idx).copied().unwrap_or(true);
            let label_pad = if label.len() + 6 < VALUE_COL {
                format!("{:<width$}", label, width = VALUE_COL - 6)
            } else {
                label.to_string()
            };

            if i == selected {
                execute!(
                    stdout,
                    Print("  "),
                    SetForegroundColor(KORA_PURPLE),
                    Print("▸ "),
                    SetForegroundColor(Color::White),
                    Print(&label_pad),
                    SetForegroundColor(KORA_PURPLE),
                    SetAttribute(Attribute::Bold),
                    Print(value),
                    SetAttribute(Attribute::Reset),
                )?;
                if !is_available {
                    execute!(stdout, SetForegroundColor(DIM), Print(" (not installed)"),)?;
                }
                execute!(stdout, ResetColor, Print("\r\n"))?;
            } else {
                let dots = ".".repeat(VALUE_COL.saturating_sub(label.len() + 6 + 1));
                execute!(
                    stdout,
                    SetForegroundColor(DIM),
                    Print(format!("    {} {}", label, dots)),
                    SetForegroundColor(Color::White),
                    Print(format!(" {}", value)),
                )?;
                if !is_available {
                    execute!(stdout, SetForegroundColor(DIM), Print(" (not installed)"),)?;
                }
                execute!(stdout, ResetColor, Print("\r\n"))?;
            }
        }

        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("  "),
            SetForegroundColor(KORA_PURPLE),
            Print("Space"),
            SetForegroundColor(DIM),
            Print(" change · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Enter"),
            SetForegroundColor(DIM),
            Print(" save · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Esc"),
            SetForegroundColor(DIM),
            Print(" cancel"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => selected = selected.saturating_sub(1),
                KeyCode::Down => {
                    if selected < last_idx {
                        selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    // Cycle to next available choice, skipping unavailable ones
                    let start = values[selected];
                    let mut next = (start + 1) % choices.len();
                    while !available.get(next).copied().unwrap_or(true) && next != start {
                        next = (next + 1) % choices.len();
                    }
                    values[selected] = next;
                }
                KeyCode::Enter => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(true);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    *values = original;
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(false);
                }
                _ => {}
            }
        }

        execute!(stdout, cursor::MoveUp(move_up as u16))?;
    }
}

/// Confirmation dialog with a warning message and Yes/No options.
/// Returns true if confirmed, false if cancelled.
pub fn confirm_action(message: &str) -> io::Result<bool> {
    let mut stdout = io::stdout();
    let mut selected: usize = 0;

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    // Layout: blank + warning + blank + 2 options + blank + hint = 6 lines with \r\n
    let move_up: usize = 6;
    let options = ["Yes, continue", "No, cancel"];

    loop {
        // Warning message
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("  "),
            SetForegroundColor(Color::Yellow),
            Print("⚠ "),
            SetForegroundColor(Color::White),
            Print(message),
            ResetColor,
            Print("\r\n"),
        )?;
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        // Options
        for (i, opt) in options.iter().enumerate() {
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;
            if i == selected {
                execute!(
                    stdout,
                    Print("  "),
                    SetForegroundColor(KORA_PURPLE),
                    Print("▸ "),
                    SetForegroundColor(Color::White),
                    Print(opt),
                    ResetColor,
                    Print("\r\n"),
                )?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(DIM),
                    Print(format!("    {}", opt)),
                    ResetColor,
                    Print("\r\n"),
                )?;
            }
        }

        // Padding + hint
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("  "),
            SetForegroundColor(KORA_PURPLE),
            Print("Enter"),
            SetForegroundColor(DIM),
            Print(" confirm · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Esc"),
            SetForegroundColor(DIM),
            Print(" cancel"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => selected = 0,
                KeyCode::Down => selected = 1,
                KeyCode::Enter | KeyCode::Char(' ') => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(selected == 0);
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(false);
                }
                _ => {}
            }
        }

        execute!(stdout, cursor::MoveUp(move_up as u16))?;
    }
}

/// Preset entry for the preset selection panel.
pub struct PresetOption {
    pub name: String,
    pub quality_bar: String,
    pub speed_bar: String,
    pub description: String,
}

/// Preset selection panel with quality/speed bars and descriptions.
/// Returns Some(index) on Enter/Space, None on Esc.
pub fn preset_panel(presets: &[PresetOption], current: usize) -> io::Result<Option<usize>> {
    let mut stdout = io::stdout();
    let mut selected = current.min(presets.len().saturating_sub(1));

    terminal::enable_raw_mode()?;
    execute!(stdout, cursor::Hide)?;

    // Each preset: name+bars line, description line = 2 lines
    // Plus 1 top padding, 1 bottom padding. Hint has no \r\n.
    let move_up = presets.len() * 2 + 2;
    let last_idx = presets.len() - 1;

    loop {
        // Top padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        for (i, preset) in presets.iter().enumerate() {
            // Line 1: name + bars (always shown)
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;
            let has_bars = !preset.quality_bar.is_empty();
            if i == selected {
                execute!(
                    stdout,
                    Print("  "),
                    SetForegroundColor(KORA_PURPLE),
                    Print("▸ "),
                    SetForegroundColor(Color::White),
                    SetAttribute(Attribute::Bold),
                    Print(&preset.name),
                    SetAttribute(Attribute::Reset),
                    ResetColor,
                )?;
                if has_bars {
                    execute!(
                        stdout,
                        Print("  "),
                        SetForegroundColor(KORA_PURPLE),
                        Print(&preset.quality_bar),
                        SetForegroundColor(DIM),
                        Print(" quality  "),
                        SetForegroundColor(KORA_PURPLE),
                        Print(&preset.speed_bar),
                        SetForegroundColor(DIM),
                        Print(" speed"),
                        ResetColor,
                    )?;
                }
                execute!(stdout, Print("\r\n"))?;
            } else {
                execute!(
                    stdout,
                    Print("    "),
                    SetForegroundColor(DIM),
                    Print(&preset.name)
                )?;
                if has_bars {
                    execute!(
                        stdout,
                        Print("  "),
                        Print(&preset.quality_bar),
                        Print(" quality  "),
                        Print(&preset.speed_bar),
                        Print(" speed"),
                    )?;
                }
                execute!(stdout, ResetColor, Print("\r\n"))?;
            }

            // Line 2: description (always shown)
            execute!(
                stdout,
                cursor::MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;
            if i == selected {
                execute!(
                    stdout,
                    SetForegroundColor(Color::White),
                    Print(format!("      {}", preset.description)),
                    ResetColor,
                    Print("\r\n")
                )?;
            } else {
                execute!(
                    stdout,
                    SetForegroundColor(DIM),
                    Print(format!("      {}", preset.description)),
                    ResetColor,
                    Print("\r\n")
                )?;
            }
        }

        // Bottom padding
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("\r\n")
        )?;

        // Hint
        execute!(
            stdout,
            cursor::MoveToColumn(0),
            Clear(ClearType::CurrentLine),
            Print("  "),
            SetForegroundColor(KORA_PURPLE),
            Print("Enter"),
            SetForegroundColor(DIM),
            Print(" select · "),
            SetForegroundColor(KORA_PURPLE),
            Print("Esc"),
            SetForegroundColor(DIM),
            Print(" cancel"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => selected = selected.saturating_sub(1),
                KeyCode::Down => {
                    if selected < last_idx {
                        selected += 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(Some(selected));
                }
                KeyCode::Esc | KeyCode::Char('q') => {
                    execute!(
                        stdout,
                        cursor::MoveUp(move_up as u16),
                        cursor::MoveToColumn(0),
                        Clear(ClearType::FromCursorDown),
                        cursor::Show
                    )?;
                    terminal::disable_raw_mode()?;
                    return Ok(None);
                }
                _ => {}
            }
        }

        execute!(stdout, cursor::MoveUp(move_up as u16))?;
    }
}
