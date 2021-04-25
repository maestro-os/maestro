use std::io::{Write, stdout};
use std::process;

use crossterm::{cursor, execute, Result, terminal::{EnterAlternateScreen, LeaveAlternateScreen}};

fn display() -> Result<()> {
	execute!(stdout(), EnterAlternateScreen)?;

	// TODO Print menu
    // TODO Listen for keyboard events

    execute!(stdout(), LeaveAlternateScreen)
}

fn main() {
	if display().is_err() {
		eprintln!("Terminal error!");
		process::exit(1);
	}
}
