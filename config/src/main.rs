/// The configuration utility allows to easily create configuration files for the kernel's
/// compilation.

mod button;
mod option;

use std::cmp::min;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::io::stdout;
use std::io;
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

use button::BackButton;
use button::Button;
use button::EnterButton;
use button::ExitButton;
use button::SaveButton;
use option::MenuOption;

/// Minimum display width.
const DISPLAY_MIN_WIDTH: u16 = 80;
/// Minimum display height.
const DISPLAY_MIN_HEIGHT: u16 = 25;

/// The path to the file containing configuration file options.
const CONFIG_OPTIONS_FILE: &str = "config_options.json";
/// The path to the output configuration file.
const CONFIG_FILE: &str = ".config";

/// Renders the `screen too small` error.
fn render_screen_error() -> Result<()> {
	execute!(stdout(),
		SetForegroundColor(Color::Black),
		SetBackgroundColor(Color::Red))?;
	execute!(stdout(), Clear(ClearType::All))?;
	execute!(stdout(), cursor::MoveTo(0, 0))?;
	println!(concat!("Display is too small! (minimum 80x25)"));
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
	print!("<Enter>: Go to menu");
	execute!(stdout(), cursor::MoveTo(menu_x + 2, menu_y + 4))?;
	print!("<Backspace>: Go to parent menu");

	let options_x = menu_x + 6;
	let options_y = menu_y + 6;
	let options_width = menu_width - options_x;
	let options_height = menu_height - options_y;
	Ok((options_x, options_y, options_width, options_height))
}

/// Represents the viewing point of a menu.
struct MenuView {
	/// The identifier of the menu.
	menu_id: String,
	/// The X position of the cursor.
	cursor_y: usize,
}

/// Structure representing the configuration environment, storage data for rendering and
/// configuration itself.
struct ConfigEnv {
	/// The list of available options in the root menu.
	options: Vec<MenuOption>,

	/// The Y position of the cursor.
	cursor_x: usize,
	/// Stores the current menu view. The last element is the view being shown.
	current_menu_view: Vec<MenuView>,
	/// The list of buttons on the interface.
	buttons: Vec<Box<dyn Button>>,

	/// Hashmap containing the current configuration values.
	/// The key is the path to the variable and value is the value associated with the variable.
	config_values: HashMap<String, String>,
}

impl ConfigEnv {
	/// Creates a new instance.
	/// `options` is the list of options.
	/// `values` is the values for all options.
	pub fn new(options: Vec<MenuOption>, values: HashMap<String, String>) -> Self {
		Self {
			options: options,

			cursor_x: 0,
			current_menu_view: vec! {
				MenuView {
					menu_id: "".to_string(),
					cursor_y: 0,
				}
			},
			buttons: vec!{
				Box::new(EnterButton {}),
				Box::new(BackButton {}),
				Box::new(SaveButton {}),
				Box::new(ExitButton {}),
			},

			config_values: values,
		}
	}

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
		let mut option: Option<&MenuOption> = None;

		for i in 1..self.current_menu_view.len() {
			option = {
				let id = self.current_menu_view[i].menu_id.clone();

				if let Some(o) = option {
					o.get_suboption(&id)
				} else {
					self.get_root_option(&id)
				}
			};

			if option.is_none() {
				panic!("Internal error");
			}
		}

		option
	}

	/// Returns the number of options in the current menu.
	fn get_current_options(&self) -> &Vec<MenuOption> {
		if let Some(menu) = &self.get_current_menu() {
			&menu.suboptions
		} else {
			&self.options
		}
	}

	/// Returns the current menu view.
	fn get_current_view(&mut self) -> &mut MenuView {
		let last = self.current_menu_view.len() - 1;
		&mut self.current_menu_view[last]
	}

	/// Renders the options. The arguments define the frame in which the options will be rendered.
	fn render_options(&mut self, opt_x: u16, opt_y: u16, opt_width: u16, opt_height: u16)
		-> Result<()> {
		let options_count = self.get_current_options().len();

		// TODO Print current menu path

		// TODO Limit rendering and add scrolling
		for i in 0..options_count {
			execute!(stdout(), cursor::MoveTo(opt_x, opt_y + i as u16))?;
			if i == self.get_current_view().cursor_y {
				execute!(stdout(),
					SetForegroundColor(Color::Grey),
					SetBackgroundColor(Color::Black))?;
			} else {
				execute!(stdout(),
					SetForegroundColor(Color::Black),
					SetBackgroundColor(Color::Grey))?;
			}

			let option = &self.get_current_options()[i];
			option.print("TODO"); // TODO Get value
		}

		execute!(stdout(),
			SetForegroundColor(Color::Black),
			SetBackgroundColor(Color::Grey))?;
		// TODO Scrolling

		Ok(())
	}

	/// Renders the options. `x` and `y` are the coordinates of the buttons.
	fn render_buttons(&mut self, x: u16, y: u16) -> Result<()> {
		execute!(stdout(), cursor::MoveTo(x, y))?;
		for i in 0..self.buttons.len() {
			if i == self.cursor_x {
				execute!(stdout(),
					SetForegroundColor(Color::Black),
					SetBackgroundColor(Color::Red))?;
			}
			print!("<{}>", self.buttons[i].get_name());

			execute!(stdout(),
				SetForegroundColor(Color::Black),
				SetBackgroundColor(Color::Grey))?;
			if i < self.buttons.len() - 1 {
				print!(" ");
			}
		}
		println!();

		Ok(())
	}

	/// Renders the menu.
	fn render(&mut self) -> Result<()> {
		let (width, height) = terminal::size()?;

		if width < DISPLAY_MIN_WIDTH || height < DISPLAY_MIN_HEIGHT {
			render_screen_error()
		} else {
			let (opt_x, opt_y, opt_width, opt_height) = render_background(width, height)?;
			self.render_options(opt_x, opt_y, opt_width, opt_height);

			let buttons_x = opt_x;
			let buttons_y = opt_y + opt_height;
			self.render_buttons(buttons_x, buttons_y);

			execute!(stdout(), cursor::MoveTo(opt_x,
				opt_y + self.get_current_view().cursor_y as u16))
		}
	}

	/// Moves the cursor up. `n` is the number of lines to move up.
	fn move_up(&mut self, mut n: usize) {
		let curr = self.get_current_view().cursor_y;
        if curr > 0 {
            self.get_current_view().cursor_y -= min(n, curr);
            self.render();
        }
	}

	/// Moves the cursor down. `n` is the number of lines to move down.
	fn move_down(&mut self, n: usize) {
		let max = self.get_current_options().len() - 1;
		let curr = self.get_current_view().cursor_y;
        if curr < max {
            self.get_current_view().cursor_y = min(curr + n, max);
            self.render();
        }
	}

	/// Moves the cursor left.
	fn move_left(&mut self) {
        if self.cursor_x > 0 {
            self.cursor_x -= 1;
            self.render();
        }
	}

	/// Moves the cursor right.
	fn move_right(&mut self) {
        if self.cursor_x < self.buttons.len() - 1 {
            self.cursor_x += 1;
            self.render();
        }
	}

	/// Moves the cursor to the top.
	fn move_top(&mut self) {
		let curr = self.get_current_view().cursor_y;
		if curr > 0 {
            self.get_current_view().cursor_y = 0;
            self.render();
		}
	}

	/// Moves the cursor to the bottom.
	fn move_bottom(&mut self) {
		let max = self.get_current_options().len() - 1;
		let curr = self.get_current_view().cursor_y;
		if curr < max {
            self.get_current_view().cursor_y = max;
            self.render();
		}
	}

	/// Toggles the selected option.
	fn toggle(&mut self) {
		// TODO
	}

	/// Performs the action of the select button.
	fn press_button(&mut self) {
		let button_index = self.cursor_x;
		// TODO Clean
		unsafe {
			let button = &mut *(&mut *self.buttons[button_index] as *mut dyn Button);
			(*button).on_action(self);
		}
	}

	/// Enters the selected menu.
	pub fn push_menu(&mut self) {
		let menu_index = self.get_current_view().cursor_y;
		let menu_type = self.get_current_options()[menu_index].option_type.clone();
		if menu_type == "menu" {
			let menu_id = self.get_current_options()[menu_index].name.clone();
			self.current_menu_view.push(MenuView {
				menu_id: menu_id,
				cursor_y: 0,
			});
			self.render();
		}
	}

	/// Goes back to the parent menu.
	pub fn pop_menu(&mut self) {
		if self.current_menu_view.len() > 1 {
			self.current_menu_view.pop();
			self.render();
		}
	}

	/// Returns the value of the variable for the given name `name`.
	fn get_value(&self, name: &String) -> String {
		if let Some(val) = self.config_values.get(name) {
			val.clone()
		} else {
			"".to_owned()
		}
	}

	/// Serializes all the options in the given options list `options` and writes into the buffer
	/// `data`. `prefix` is the prefix of the variables to create.
	fn serialize_menu(&self, prefix: &String, options: &Vec<MenuOption>, data: &mut String) {
		for o in options {
			if o.option_type == "menu" {
				let new_prefix = prefix.clone() + &o.name + "_";
				self.serialize_menu(&new_prefix, &o.suboptions, data);
			} else {
				let name = prefix.clone() + &o.name;
				let value = self.get_value(&name);
				*data = data.clone() + &name + "=\"" + &value + "\"\n";
			}
		}
	}

	/// Saves configuration to file.
	pub fn save(&self) -> io::Result<()> {
		let mut data = String::new();
		self.serialize_menu(&"".to_owned(), &self.options, &mut data);

		let mut file = File::create(CONFIG_FILE)?;
		file.write_all(data.as_str().as_bytes())
	}
}

/// Resets the terminal before quitting.
fn reset() -> Result<()> {
	terminal::disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)
}

fn exit() -> Result<()> {
    // TODO Ask for confirmation
	reset()?;
    process::exit(0);
}

/// Waits for a keyboard or resize event.
fn wait_for_event(env: &mut ConfigEnv) -> Result<()> {
	loop {
        match event::read()? {
            Event::Key(event) => {
            	match event.code {
            		KeyCode::Up | KeyCode::Char('k') => {
            			env.move_up(1);
            		},
            		KeyCode::Down | KeyCode::Char('j') => {
            			env.move_down(1);
            		},
            		KeyCode::Left | KeyCode::Char('h') => {
            			env.move_left();
            		},
            		KeyCode::Right => {
            			env.move_right();
            		},

            		KeyCode::Char('l') => {
            			if event.modifiers == KeyModifiers::CONTROL {
            				env.render();
            			} else {
            				env.move_right();
            			}
            		},

            		KeyCode::PageUp => {
            			env.move_up(10);
            		},
            		KeyCode::PageDown => {
            			env.move_down(10);
            		},
            		KeyCode::Home => {
            			env.move_top();
            		},
            		KeyCode::End => {
            			env.move_bottom();
            		},

            		KeyCode::Char(' ') => {
            			env.toggle();
            		},
            		KeyCode::Enter => {
            			env.press_button();
            		},
            		KeyCode::Backspace => {
            			env.pop_menu();
            		},
            		KeyCode::Esc => {
            			env.pop_menu();
            		},

            		_ => {},
            	}
            },

            Event::Resize(_, _) => {
            	env.render();
            },

            _ => {},
        }
    }
}

/// Displays the configuration utility.
fn display(options: Vec<MenuOption>, values: HashMap<String, String>) -> Result<()> {
	execute!(stdout(), EnterAlternateScreen)?;
	terminal::enable_raw_mode()?;

	let mut env = ConfigEnv::new(options, values);
	env.render();
    wait_for_event(&mut env)?;

	reset()
}

/// TODO doc
fn get_values_(prefix: &String, options: &Vec<MenuOption>, values: &mut HashMap<String, String>) {
	for o in options {
		if o.option_type == "menu" {
			let new_prefix = prefix.clone() + &o.name + "_";
			get_values_(&new_prefix, &o.suboptions, values);
		} else {
			let name = prefix.clone() + &o.name;
			let value = &o.default; // TODO If config file exists, read from it
			values.insert(name, value.to_string());
		}
	}
}

/// Returns a hash map containing the values for all options according to the config file if it
/// exists, or the default values.
fn get_values(options: &Vec<MenuOption>) -> HashMap<String, String> {
	let mut map = HashMap::new();
	get_values_(&"".to_owned(), options, &mut map);
	map
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

	if width < DISPLAY_MIN_WIDTH || height < DISPLAY_MIN_HEIGHT {
		eprintln!(concat!("The terminal must be at least 80x25 characters in size to run the
configuration tool"));
		process::exit(1);
	}

	let options_results = option::from_file(CONFIG_OPTIONS_FILE);
	if let Err(err) = options_results {
		eprintln!("{}", err);
		process::exit(1);
	}
	let options = options_results.unwrap();
	let values = get_values(&options);

	if display(options, values).is_err() {
		eprintln!("Terminal error!");
		process::exit(1);
	}
}
