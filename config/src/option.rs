/// This module implements the MenuOption structure and its parsing from a JSON file.
/// An option can either be a menu, or a configuration option to select.

use std::fs;

use serde::Deserialize;

/// Structure representing an option.
#[derive(Deserialize)]
pub struct MenuOption {
	/// The identifier of the option.
	pub name: String,
	/// The display name of the option.
	pub display_name: String,
	/// The description of the option.
	pub desc: String,
	/// The type of the option.
	pub option_type: String,
	/// The set of possible values.
	pub values: Vec<String>,
	/// The default value.
	pub default: String,
	/// The set of dependencies needed to enable this option.
	pub deps: Vec<String>,
	/// The set of suboptions.
	pub suboptions: Vec<MenuOption>,
}

impl MenuOption {
	/// Returns the option with name `name` within the current menu.
	pub fn get_suboption(&self, name: &String) -> Option<&MenuOption> {
		for m in &self.suboptions {
			if m.name == *name {
				return Some(&m);
			}
		}

		None
	}

	/// Prints the entry to the screen.
	/// `value` is the current value of the entry.
	pub fn print(&self, value: &str) {
		match self.option_type.as_str() {
			"menu" => {
				println!("-> {}", self.display_name);
			},
			"choice" | "bool" => {
				println!("[{}] {}", value, self.display_name);
			}

			_ => {
				println!("!!! INVALID OPTION !!!");
			}
		}
	}
}

/// Reads all options from the given JSON file `file`.
pub fn from_file(file: &str) -> Result<Vec<MenuOption>, &'static str> {
	if let Ok(data) = fs::read_to_string(file) {
		let options_result = serde_json::from_str(&data);
		if let Err(err) = options_result {
			eprintln!("{}", err); // TODO Move?
			Err("Failed to parse options file!")
		} else {
			// TODO Check for dependencies cycle
			Ok(options_result.unwrap())
		}
	} else {
		Err("Failed to open options file!")
	}
}
