/// The configuration utility allows to easily create configuration files for the kernel's
/// compilation.

use std::io::stdout;
use std::process;

use crossterm::Result;
use crossterm::cursor;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyModifiers;
use crossterm::event;
use crossterm::execute;
use crossterm::style::Color;
use crossterm::style::SetBackgroundColor;
use crossterm::style::SetForegroundColor;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use crossterm::terminal;
use crossterm::tty::IsTty;

/// Structure representing the configuration environment, storage data for rendering and
/// configuration itself.
struct ConfigEnv {
	cursor_x: usize,
	cursor_y: usize,

	// TODO
}

impl ConfigEnv {
	/// Returns the number of options in the current menu.
	fn get_options_count(&self) -> usize {
		// TODO
		10
	}

	/// Returns the number of buttons.
	fn get_buttons_count(&self) -> usize {
		// TODO
		10
	}
}

/// Renders the menu.
fn render_menu(env: &mut ConfigEnv) -> Result<()> {
	let (width, height) = terminal::size()?;

	execute!(stdout(),
		SetForegroundColor(Color::Black),
		SetBackgroundColor(Color::Blue))?;
	execute!(stdout(), Clear(ClearType::All))?;

	execute!(stdout(), cursor::MoveTo(1, 0))?;
	print!("Maestro kernel configuration utility");

	let menu_x = 2;
	let menu_y = 2;
	let menu_width = width - (menu_x * 2);
	let menu_height = height - (menu_y * 2);
	execute!(stdout(),
		SetForegroundColor(Color::Black),
		SetBackgroundColor(Color::Grey))?;
	for i in 0..menu_height {
		execute!(stdout(), cursor::MoveTo(menu_x, menu_y + i))?;
		for j in 0..menu_width {
			if j == 0 || j == menu_width - 1 {
				print!("|");
			} else if i == 0 || i == menu_height - 1 {
				print!("-");
			} else {
				print!(" ");
			}
		}
	}

	execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 1))?;
	print!("<Up>/<Down>/<Left>/<Right>: Move around");
	execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 2))?;
	print!("<Space>: Toggle");
	execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 3))?;
	print!("<Enter>: Validate");

	execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 5))?;
	// TODO Render options

	// TODO Render buttons

	execute!(stdout(), cursor::MoveTo(env.cursor_x as _, env.cursor_y as _))
}

// TODO rm?
fn update_selection(env: &mut ConfigEnv) -> Result<()> {
	// TODO
	execute!(stdout(), cursor::MoveTo(env.cursor_x as _, env.cursor_y as _))
}

/// Resets the terminal before quitting.
fn reset() {
	terminal::disable_raw_mode();
    execute!(stdout(), LeaveAlternateScreen);
}

/// Waits for a keyboard or resize event.
fn wait_for_event(env: &mut ConfigEnv) -> Result<()> {
	loop {
        match event::read()? {
            Event::Key(event) => {
            	match event.code {
            		KeyCode::Up | KeyCode::Char('k') => {
            			if env.cursor_y > 0 {
            				env.cursor_y -= 1;
            				update_selection(env);
            			}
            		},
            		KeyCode::Down | KeyCode::Char('j') => {
            			if env.cursor_y < env.get_options_count() {
            				env.cursor_y += 1;
            				update_selection(env);
            			}
            		},
            		KeyCode::Left | KeyCode::Char('h') => {
            			if env.cursor_x > 0 {
            				env.cursor_x -= 1;
            				update_selection(env);
            			}
            		},
            		KeyCode::Right | KeyCode::Char('l') => {
            			if env.cursor_x < env.get_buttons_count() {
            				env.cursor_x += 1;
            				update_selection(env);
            			}
            		},

            		KeyCode::PageUp => {
            			// TODO
            			println!("PageUp");
            		},
            		KeyCode::PageDown => {
            			// TODO
            			println!("PageDown");
            		},

            		KeyCode::Char(' ') => {
            			// TODO
            			println!("Space");
            		},
            		KeyCode::Enter => {
            			// TODO
            			println!("Enter");
            		},

            		KeyCode::Char('l') => {
            			if event.modifiers == KeyModifiers::CONTROL {
            				render_menu(env);
            			}
            		},

            		KeyCode::Esc => {
            			// TODO Ask for confirmation
						reset();
            			process::exit(0);
            		},

            		_ => {},
            	}
            },
            Event::Resize(width, height) => println!("New size {}x{}", width, height), // TODO

            _ => {},
        }
    }
}

/// Displays the configuration utility.
fn display() -> Result<()> {
	execute!(stdout(), EnterAlternateScreen)?;
	terminal::enable_raw_mode();

	let mut env = ConfigEnv {
		cursor_x: 0,
		cursor_y: 0,
	};

	render_menu(&mut env);
    wait_for_event(&mut env)?;

	reset();
	Ok(())
}

fn main() {
	let s = stdout();

	if !s.is_tty() {
		eprintln!("Standard output must be a terminal!");
		process::exit(1);
	}

	let size = terminal::size();
	if size.is_err() {
		eprintln!("Cannot retrieve terminal size!");
		process::exit(1);
	}
	let (width, height) = size.unwrap();

	if width < 80 || height < 25 {
		eprintln!("The terminal must be at least 80x25 characters in size to run the configuration
tool");
		process::exit(1);
	}

	if display().is_err() {
		eprintln!("Terminal error!");
		process::exit(1);
	}
}
