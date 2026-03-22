use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal,
};
use std::io::{self, Write};

pub fn select(prompt: &str, options: &[&str]) -> io::Result<usize> {
    let mut stdout = io::stdout();
    let mut selected = 0usize;

    execute!(stdout, Print(format!("\n  ? {}\n\n", prompt)))?;

    terminal::enable_raw_mode()?;
    let result = select_inner(options, &mut stdout, &mut selected);
    terminal::disable_raw_mode()?;

    result?;
    execute!(stdout, Print("\n"))?;
    Ok(selected)
}

fn select_inner(options: &[&str], stdout: &mut io::Stdout, selected: &mut usize) -> io::Result<()> {
    loop {
        for (i, option) in options.iter().enumerate() {
            execute!(*stdout, cursor::MoveToColumn(0))?;
            if i == *selected {
                execute!(
                    *stdout,
                    SetForegroundColor(Color::Cyan),
                    Print(format!("    > {}\n", option)),
                    ResetColor,
                )?;
            } else {
                execute!(*stdout, Print(format!("      {}\n", option)))?;
            }
        }

        execute!(
            *stdout,
            Print("\n"),
            SetForegroundColor(Color::DarkGrey),
            Print("                                          up/down navigate, enter select"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    if *selected < options.len() - 1 {
                        *selected += 1;
                    }
                }
                KeyCode::Enter => return Ok(()),
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                _ => {}
            }
        }

        let lines_to_clear = options.len() + 2;
        execute!(*stdout, cursor::MoveUp(lines_to_clear as u16))?;
    }
}

pub fn multi_select(prompt: &str, options: &[&str]) -> io::Result<Vec<usize>> {
    let mut stdout = io::stdout();
    let mut selected = 0usize;
    let mut toggled = vec![false; options.len()];

    execute!(stdout, Print(format!("\n  ? {}\n\n", prompt)))?;

    terminal::enable_raw_mode()?;
    let result = multi_select_inner(options, &mut stdout, &mut selected, &mut toggled);
    terminal::disable_raw_mode()?;

    result?;
    execute!(stdout, Print("\n"))?;

    Ok(toggled
        .iter()
        .enumerate()
        .filter(|(_, t)| **t)
        .map(|(i, _)| i)
        .collect())
}

fn multi_select_inner(
    options: &[&str],
    stdout: &mut io::Stdout,
    selected: &mut usize,
    toggled: &mut [bool],
) -> io::Result<()> {
    loop {
        for (i, option) in options.iter().enumerate() {
            execute!(*stdout, cursor::MoveToColumn(0))?;
            let marker = if toggled[i] { "(*)" } else { "( )" };
            if i == *selected {
                execute!(
                    *stdout,
                    SetForegroundColor(Color::Cyan),
                    Print(format!("    {} {}\n", marker, option)),
                    ResetColor,
                )?;
            } else {
                execute!(*stdout, Print(format!("    {} {}\n", marker, option)))?;
            }
        }

        execute!(
            *stdout,
            Print("\n"),
            SetForegroundColor(Color::DarkGrey),
            Print("                              up/down navigate, space toggle, enter confirm"),
            ResetColor,
        )?;

        stdout.flush()?;

        if let Event::Key(KeyEvent { code, .. }) = event::read()? {
            match code {
                KeyCode::Up => {
                    *selected = selected.saturating_sub(1);
                }
                KeyCode::Down => {
                    if *selected < options.len() - 1 {
                        *selected += 1;
                    }
                }
                KeyCode::Char(' ') => {
                    toggled[*selected] = !toggled[*selected];
                }
                KeyCode::Enter | KeyCode::Esc => return Ok(()),
                _ => {}
            }
        }

        let lines_to_clear = options.len() + 2;
        execute!(*stdout, cursor::MoveUp(lines_to_clear as u16))?;
    }
}
