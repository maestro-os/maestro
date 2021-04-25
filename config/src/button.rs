use crate::ConfigEnv;

/// Trait representing a button on the interface.
pub trait Button {
	/// Returns the name of the button.
	fn get_name(&self) -> String;
	/// Function called on action on the button.
	fn on_action(&mut self, _env: &mut ConfigEnv);
}

/// Structure representing the Enter button. On action, enters the select menu.
pub struct EnterButton {}

impl Button for EnterButton {
	fn get_name(&self) -> String {
		"Enter".to_string()
	}

	fn on_action(&mut self, env: &mut ConfigEnv) {
		env.push_menu();
	}
}

/// Structure representing the Back button. On action, goes back to the parent menu.
pub struct BackButton {}

impl Button for BackButton {
	fn get_name(&self) -> String {
		"Back".to_string()
	}

	fn on_action(&mut self, env: &mut ConfigEnv) {
		env.pop_menu();
	}
}

/// Structure representing the Save button. On action, saves the configuration.
pub struct SaveButton {}

impl Button for SaveButton {
	fn get_name(&self) -> String {
		"Save".to_string()
	}

	fn on_action(&mut self, _env: &mut ConfigEnv) {
		// TODO
	}
}

/// Structure representing the Exit button. On action, exits the program.
pub struct ExitButton {}

impl Button for ExitButton {
	fn get_name(&self) -> String {
		"Exit".to_string()
	}

	fn on_action(&mut self, _env: &mut ConfigEnv) {
		crate::exit();
	}
}
