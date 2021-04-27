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
	/// The value of the option.
	pub value: String,
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
				println!(" -> {}", self.display_name);
			},
			_ => {
				println!(" [{}] {}", value, self.display_name);
			}
		}
	}

	/// Serializes the current menu and submenus and writes into the buffer `data`.
	/// `prefix` is the prefix of the variables to create.
	pub fn serialize(&self, prefix: &String, data: &mut String) {
		if self.option_type != "menu" {
			let name = prefix.clone() + &self.name;
			let value = &self.value;
			*data = data.clone() + &name + "=\"" + value + "\"\n";
		}

		let new_prefix = prefix.clone() + &self.name + "_";
		for o in &self.suboptions {
			o.serialize(&new_prefix, data);
		}
	}
}

/// Changes options names to full pathes.
fn translate_names(prefix: &String, options: &mut Vec<MenuOption>) {
	for o in options {
		if o.option_type == "menu" {
			let new_prefix = prefix.clone() + &o.name + "_";
			translate_names(&new_prefix, &mut o.suboptions);
		} else {
			let new_name = prefix.clone() + &o.name;
			o.name = new_name;
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
			let mut options = options_result.unwrap();
			translate_names(&"".to_owned(), &mut options);
			// TODO Check for dependencies cycle
			Ok(options)
		}
	} else {
		Err("Failed to open options file!")
	}
}
