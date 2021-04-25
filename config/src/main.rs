/// The configuration utility allows to easily create configuration files for the kernel's
/// compilation.

mod option;

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

use option::MenuOption;

/// Structure representing the configuration environment, storage data for rendering and
/// configuration itself.
struct ConfigEnv {
	/// The X position of the cursor.
	cursor_x: usize,
	/// The Y position of the cursor.
	cursor_y: usize,

	/// The list of available options in the root menu.
	options: Vec<MenuOption>,

	/// Stores the current menu.
	current_menu: Vec<String>,
}

impl ConfigEnv {
	/// Returns the option with name `name` within the root menu.
	fn get_root_option(&self, name: &String) -> Option<&MenuOption> {
		for m in &self.options {
			if m.name == *name {
				return Some(&m);
			}
		}

		None
	}

	/// Returns the current menu.
	fn get_current_menu(&self) -> Option<&MenuOption> {
		let mut menu: Option<&MenuOption> = None;
		for m in &self.current_menu {
			menu = {
				if let Some(menu_) = menu {
					menu_.get_suboption(&m)
				} else {
					self.get_root_option(&m)
				}
			};

			if menu.is_none() {
				// TODO Error
				return None;
			}
		}
		menu
	}

	/// Returns the number of options in the current menu.
	fn get_current_options(&self) -> &Vec<MenuOption> {
		if let Some(menu) = &self.get_current_menu() {
			&menu.suboptions
		} else {
			&self.options
		}
	}

	/// Returns the number of buttons.
	fn get_buttons_count(&self) -> usize {
		// TODO
		10
	}

	/// Moves the cursor up.
	fn move_up(&mut self) {
        if self.cursor_y > 0 {
            self.cursor_y -= 1;
            render_menu(self);
        }
	}

	/// Moves the cursor down.
	fn move_down(&mut self) {
        if self.cursor_y < self.get_current_options().len() - 1 {
            self.cursor_y += 1;
            render_menu(self);
        }
	}

	/// Moves the cursor left.
	fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
            render_menu(self);
        }
	}

	/// Moves the cursor right.
	fn move_right(&mut self) {
        if self.cursor_x < self.get_buttons_count() - 1{
            self.cursor_x += 1;
            render_menu(self);
        }
	}
}

/// Renders the `screen too small` error.
fn render_screen_error() -> Result<()> {
	execute!(stdout(),
		SetForegroundColor(Color::Black),
		SetBackgroundColor(Color::Red))?;
	execute!(stdout(), Clear(ClearType::All))?;
	execute!(stdout(), cursor::MoveTo(0, 0))?;
	println!("Display is too small! (minimum 80x25)");
	execute!(stdout(), cursor::MoveTo(0, 1))
}

/// Renders the menu's background and x/y/width/height for the options.
/// `width` is the width of the terminal.
/// `height` is the height of the terminal.
fn render_background(width: u16, height: u16) -> Result<(u16, u16, u16, u16)> {
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

	let options_x = menu_x + 5;
	let options_y = menu_y + 5;
	let options_width = 100; // TODO
	let options_height = 100; // TODO
	Ok((options_x, options_y, options_width, options_height))
}

/// Renders the menu.
fn render_menu(env: &mut ConfigEnv) -> Result<()> {
	let (width, height) = terminal::size()?;

	if width < 80 || height < 25 {
		render_screen_error()
	} else {
		let (opt_x, opt_y, opt_width, opt_height) = render_background(width, height)?;
		for i in 0..env.get_current_options().len() {
			execute!(stdout(), cursor::MoveTo(opt_x, opt_y + i as u16))?;

			let option = &env.get_current_options()[i];
			if option.option_type == "menu" {
				println!("{}", option.display_name); // TODO
			}
		}
		// TODO Scrolling

		// TODO Render buttons

		execute!(stdout(), cursor::MoveTo(opt_x, opt_y + env.cursor_y as u16))
	}
}

/// Resets the terminal before quitting.
fn reset() -> Result<()> {
	terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)
}

/// Waits for a keyboard or resize event.
fn wait_for_event(env: &mut ConfigEnv) -> Result<()> {
	loop {
        match event::read()? {
            Event::Key(event) => {
            	match event.code {
            		KeyCode::Up | KeyCode::Char('k') => {
            			env.move_up();
            		},
            		KeyCode::Down | KeyCode::Char('j') => {
            			env.move_down();
            		},
            		KeyCode::Left | KeyCode::Char('h') => {
            			env.move_left();
            		},
            		KeyCode::Right => {
            			env.move_right();
            		},

            		KeyCode::Char('l') => {
            			if event.modifiers == KeyModifiers::CONTROL {
            				render_menu(env);
            			} else {
            				env.move_right();
            			}
            		},

            		KeyCode::PageUp => {
            			// TODO
            		},
            		KeyCode::PageDown => {
            			// TODO
            		},

            		KeyCode::Char(' ') => {
            			// TODO
            		},
            		KeyCode::Enter => {
            			// TODO
            		},

            		KeyCode::Esc => {
            			// TODO Ask for confirmation
						reset()?;
            			process::exit(0);
            		},

            		_ => {},
            	}
            },

            Event::Resize(_, _) => {
            	render_menu(env);
            },

            _ => {},
        }
    }
}

/// Displays the configuration utility.
fn display(options: Vec<MenuOption>) -> Result<()> {
	execute!(stdout(), EnterAlternateScreen)?;
	terminal::enable_raw_mode()?;

	let mut env = ConfigEnv {
		cursor_x: 0,
		cursor_y: 0,

		options: options,

		current_menu: Vec::new(),
	};

	render_menu(&mut env);
    wait_for_event(&mut env)?;

	reset()
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

	let options_results = option::from_file("config_options.json");
	if let Err(err) = options_results {
		eprintln!("{}", err);
		process::exit(1);
	}

	let options = options_results.unwrap();
	if display(options).is_err() {
		eprintln!("Terminal error!");
		process::exit(1);
	}
}
