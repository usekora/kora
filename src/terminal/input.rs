use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{self, Write};

pub fn read_line(prompt: &str) -> io::Result<String> {
    let mut stdout = io::stdout();

    execute!(
        stdout,
        SetForegroundColor(Color::Cyan),
        Print(prompt),
        ResetColor,
    )?;
    stdout.flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

pub fn read_user_input() -> io::Result<String> {
    read_line("> ")
}
