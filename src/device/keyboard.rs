//! This module implements the keyboard device manager.

use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::device::ps2;
use crate::errno::Errno;
use crate::errno;
use crate::tty;

/// Enumation of keyboard keys.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyboardKey {
	KeyEsc,
	Key1,
	Key2,
	Key3,
	Key4,
	Key5,
	Key6,
	Key7,
	Key8,
	Key9,
	Key0,
	KeyMinus,
	KeyEqual,
	KeyBackspace,
	KeyTab,
	KeyQ,
	KeyW,
	KeyE,
	KeyR,
	KeyT,
	KeyY,
	KeyU,
	KeyI,
	KeyO,
	KeyP,
	KeyOpenBrace,
	KeyCloseBrace,
	KeyEnter,
	KeyLeftControl,
	KeyA,
	KeyS,
	KeyD,
	KeyF,
	KeyG,
	KeyH,
	KeyJ,
	KeyK,
	KeyL,
	KeySemiColon,
	KeySingleQuote,
	KeyBackTick,
	KeyLeftShift,
	KeyBackslash,
	KeyZ,
	KeyX,
	KeyC,
	KeyV,
	KeyB,
	KeyN,
	KeyM,
	KeyComma,
	KeyDot,
	KeySlash,
	KeyRightShift,
	KeyKeypadStar,
	KeyLeftAlt,
	KeySpace,
	KeyCapsLock,
	KeyF1,
	KeyF2,
	KeyF3,
	KeyF4,
	KeyF5,
	KeyF6,
	KeyF7,
	KeyF8,
	KeyF9,
	KeyF10,
	KeyNumberLock,
	KeyScrollLock,
	KeyKeypad7,
	KeyKeypad8,
	KeyKeypad9,
	KeyKeypadMinus,
	KeyKeypad4,
	KeyKeypad5,
	KeyKeypad6,
	KeyKeypadPlus,
	KeyKeypad1,
	KeyKeypad2,
	KeyKeypad3,
	KeyKeypad0,
	KeyKeypadDot,
	KeyF11,
	KeyF12,

	KeyPreviousTrack,
	KeyNextTrack,
	KeyKeypadEnter,
	KeyRightControl,
	KeyMute,
	KeyCalculator,
	KeyPlay,
	KeyStop,
	KeyVolumeDown,
	KeyVolumeUp,
	KeyWWWHome,
	KeyKeypadSlash,
	KeyRightAlt,
	KeyHome,
	KeyCursorUp,
	KeyPageUp,
	KeyCursorLeft,
	KeyCursorRight,
	KeyEnd,
	KeyCursorDown,
	KeyPageDown,
	KeyInsert,
	KeyDelete,
	KeyLeftGUI,
	KeyRightGUI,
	KeyApps,
	KeyACPIPower,
	KeyACPISleep,
	KeyACPIWake,
	KeyWWWSearch,
	KeyWWWFavorites,
	KeyWWWRefresh,
	KeyWWWStop,
	KeyWWWForward,
	KeyWWWBack,
	KeyMyComputer,
	KeyEmail,
	KeyMediaSelect,

	KeyPrintScreen,
	KeyPause,

	KeyUnknown,
}

impl KeyboardKey {
	// TODO Take ctrl state, etc...
	// TODO Re-check everything
	/// Returns the TTY characters for the given current.
	/// `shift` tells whether shift is pressed. This value has to be inverted if caps lock is
	/// enabled.
	pub fn get_tty_chars(&self, shift: bool) -> Option<&str> {
		if !shift {
			match self {
				Self::KeyEsc => Some("^["),
				Self::Key1 => Some("1"),
				Self::Key2 => Some("2"),
				Self::Key3 => Some("3"),
				Self::Key4 => Some("4"),
				Self::Key5 => Some("5"),
				Self::Key6 => Some("6"),
				Self::Key7 => Some("7"),
				Self::Key8 => Some("8"),
				Self::Key9 => Some("9"),
				Self::Key0 => Some("0"),
				Self::KeyMinus => Some("-"),
				Self::KeyEqual => Some("="),
				Self::KeyBackspace => Some("\x08"),
				Self::KeyTab => Some("\t"),
				Self::KeyQ => Some("q"),
				Self::KeyW => Some("w"),
				Self::KeyE => Some("e"),
				Self::KeyR => Some("r"),
				Self::KeyT => Some("t"),
				Self::KeyY => Some("y"),
				Self::KeyU => Some("u"),
				Self::KeyI => Some("i"),
				Self::KeyO => Some("o"),
				Self::KeyP => Some("p"),
				Self::KeyOpenBrace => Some("["),
				Self::KeyCloseBrace => Some("]"),
				Self::KeyEnter => Some("\n"),
				Self::KeyA => Some("a"),
				Self::KeyS => Some("s"),
				Self::KeyD => Some("d"),
				Self::KeyF => Some("f"),
				Self::KeyG => Some("g"),
				Self::KeyH => Some("h"),
				Self::KeyJ => Some("j"),
				Self::KeyK => Some("k"),
				Self::KeyL => Some("l"),
				Self::KeySemiColon => Some(";"),
				Self::KeySingleQuote => Some("'"),
				Self::KeyBackTick => Some("`"),
				Self::KeyBackslash => Some("\\"),
				Self::KeyZ => Some("z"),
				Self::KeyX => Some("x"),
				Self::KeyC => Some("c"),
				Self::KeyV => Some("v"),
				Self::KeyB => Some("b"),
				Self::KeyN => Some("n"),
				Self::KeyM => Some("m"),
				Self::KeyComma => Some(","),
				Self::KeyDot => Some("."),
				Self::KeySlash => Some("/"),
				Self::KeyKeypadStar => Some("*"),
				Self::KeySpace => Some(" "),
				Self::KeyF1 => Some("^[[[A"),
				Self::KeyF2 => Some("^[[[B"),
				Self::KeyF3 => Some("^[[[C"),
				Self::KeyF4 => Some("^[[[D"),
				Self::KeyF5 => Some("^[[[E"),
				Self::KeyF6 => Some("^[[17"),
				Self::KeyF7 => Some("^[[18"),
				Self::KeyF8 => Some("^[[19"),
				Self::KeyF9 => Some("^[[20"),
				Self::KeyF10 => Some("^[[21"),
				Self::KeyKeypad7 => Some("7"),
				Self::KeyKeypad8 => Some("8"),
				Self::KeyKeypad9 => Some("9"),
				Self::KeyKeypadMinus => Some("-"),
				Self::KeyKeypad4 => Some("4"),
				Self::KeyKeypad5 => Some("5"),
				Self::KeyKeypad6 => Some("6"),
				Self::KeyKeypadPlus => Some("+"),
				Self::KeyKeypad1 => Some("1"),
				Self::KeyKeypad2 => Some("2"),
				Self::KeyKeypad3 => Some("3"),
				Self::KeyKeypad0 => Some("0"),
				Self::KeyKeypadDot => Some("."),
				Self::KeyF11 => Some("^[[23~"),
				Self::KeyF12 => Some("^[[24~"),

				Self::KeyKeypadEnter => Some("\n"),
				Self::KeyKeypadSlash => Some("/"),
				Self::KeyHome => Some("^[[1~"),
				Self::KeyCursorUp => Some("^[[A"),
				Self::KeyPageUp => Some("^[[5~"),
				Self::KeyCursorLeft => Some("^[[C"),
				Self::KeyCursorRight => Some("^[[D"),
				Self::KeyEnd => Some("^[[4~"),
				Self::KeyCursorDown => Some("^[[B"),
				Self::KeyPageDown => Some("^[[6~"),

				_ => None,
			}
		} else {
			match self {
				Self::KeyEsc => Some("^["),
				Self::Key1 => Some("!"),
				Self::Key2 => Some("@"),
				Self::Key3 => Some("#"),
				Self::Key4 => Some("$"),
				Self::Key5 => Some("%"),
				Self::Key6 => Some("^"),
				Self::Key7 => Some("&"),
				Self::Key8 => Some("*"),
				Self::Key9 => Some("("),
				Self::Key0 => Some(")"),
				Self::KeyMinus => Some("_"),
				Self::KeyEqual => Some("+"),
				Self::KeyBackspace => Some("\x08"),
				Self::KeyTab => Some("\t"),
				Self::KeyQ => Some("Q"),
				Self::KeyW => Some("W"),
				Self::KeyE => Some("E"),
				Self::KeyR => Some("R"),
				Self::KeyT => Some("T"),
				Self::KeyY => Some("Y"),
				Self::KeyU => Some("U"),
				Self::KeyI => Some("I"),
				Self::KeyO => Some("O"),
				Self::KeyP => Some("P"),
				Self::KeyOpenBrace => Some("{"),
				Self::KeyCloseBrace => Some("}"),
				Self::KeyEnter => Some("\n"),
				Self::KeyA => Some("A"),
				Self::KeyS => Some("S"),
				Self::KeyD => Some("D"),
				Self::KeyF => Some("F"),
				Self::KeyG => Some("G"),
				Self::KeyH => Some("H"),
				Self::KeyJ => Some("J"),
				Self::KeyK => Some("K"),
				Self::KeyL => Some("L"),
				Self::KeySemiColon => Some(":"),
				Self::KeySingleQuote => Some("\""),
				Self::KeyBackTick => Some("~"),
				Self::KeyBackslash => Some("|"),
				Self::KeyZ => Some("Z"),
				Self::KeyX => Some("X"),
				Self::KeyC => Some("C"),
				Self::KeyV => Some("V"),
				Self::KeyB => Some("B"),
				Self::KeyN => Some("N"),
				Self::KeyM => Some("M"),
				Self::KeyComma => Some("<"),
				Self::KeyDot => Some(">"),
				Self::KeySlash => Some("?"),
				Self::KeyKeypadStar => Some("*"),
				Self::KeySpace => Some(" "),
				// TODO F1 to F10
				Self::KeyKeypad7 => Some("7"),
				Self::KeyKeypad8 => Some("8"),
				Self::KeyKeypad9 => Some("9"),
				Self::KeyKeypadMinus => Some("-"),
				Self::KeyKeypad4 => Some("4"),
				Self::KeyKeypad5 => Some("5"),
				Self::KeyKeypad6 => Some("6"),
				Self::KeyKeypadPlus => Some("+"),
				Self::KeyKeypad1 => Some("1"),
				Self::KeyKeypad2 => Some("2"),
				Self::KeyKeypad3 => Some("3"),
				Self::KeyKeypad0 => Some("0"),
				Self::KeyKeypadDot => Some("."),
				// TODO F11 and F12

				// TODO
				// Self::KeyKeypadEnter => Some("\n"),
				// Self::KeyKeypadSlash => Some("/"),
				// Self::KeyHome => Some("^[[1~"),
				// Self::KeyCursorUp => Some("^[[A"),
				// Self::KeyPageUp => Some("^[[5~"),
				// Self::KeyCursorLeft => Some("^[[C"),
				// Self::KeyCursorRight => Some("^[[D"),
				// Self::KeyEnd => Some("^[[4~"),
				// Self::KeyCursorDown => Some("^[[B"),
				// Self::KeyPageDown => Some("^[[6~"),

				_ => None,
			}
		}
	}
}

/// Enumeration of keyboard actions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyboardAction {
	/// The key was pressed.
	Pressed,
	/// The key was released.
	Released,
}

/// Enumeration of keyboard LEDs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyboardLED {
	/// The number lock LED.
	NumberLock,
	/// The caps lock LED.
	CapsLock,
	/// The scroll lock LED.
	ScrollLock,

	// TODO Add the japanese keyboard Kana mode
}

/// Structure representing a key that can enabled, such as caps lock. Such a key is usually
/// associated with an LED on the keyboard.
pub struct EnableKey {
	/// The key's state.
	state: bool,
	/// Tells whether to ignore the next pressed actions until a release action is received.
	ignore: bool,
}

impl EnableKey {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			state: false,
			ignore: false,
		}
	}

	/// Handles a keyboard input.
	/// `kbd_manager` is the keyboard manager.
	/// If the state changed, the function returns true.
	pub fn input(&mut self, action: KeyboardAction) -> bool {
		match action {
			KeyboardAction::Pressed => {
				if !self.ignore {
					self.state = !self.state;
					self.ignore = true;

					return true;
				}
			},

			KeyboardAction::Released => {
				self.ignore = false;
			},
		}

		false
	}

	/// Tells whether the key is enabled.
	pub fn is_enabled(&self) -> bool {
		self.state
	}
}

/// Trait representing a keyboard.
pub trait Keyboard {
	/// Sets the state of the given LED.
	/// `led` is the LED.
	/// `enabled` tells whether the LED is enabled.
	fn set_led(&mut self, led: KeyboardLED, enabled: bool);
}

/// Structure managing keyboard devices.
/// The manager has the name `kbd`.
pub struct KeyboardManager {
	/// The PS/2 keyboard.
	ps2_keyboard: Option<ps2::PS2Keyboard>,

	/// The ctrl key state.
	ctrl: bool,
	/// The shift key state.
	shift: bool,
	/// The alt key state.
	alt: bool,
	/// The right alt key state.
	right_alt: bool,
	/// The right ctrl key state.
	right_ctrl: bool,

	/// The number lock state.
	number_lock: EnableKey,
	/// The caps lock state.
	caps_lock: EnableKey,
	/// The scroll lock state.
	scroll_lock: EnableKey,
}

impl KeyboardManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		let s = Self {
			ps2_keyboard: None,

			ctrl: false,
			shift: false,
			alt: false,
			right_alt: false,
			right_ctrl: false,

			number_lock: EnableKey::new(),
			caps_lock: EnableKey::new(),
			scroll_lock: EnableKey::new(),
		};
		s.init_device_files();
		s
	}

	/// Initializes devices files.
	fn init_device_files(&self) {
		// TODO Create /dev/input/event* files
	}

	/// Destroyes devices files.
	fn fini_device_files(&self) {
		// TODO Remove /dev/input/event* files
	}

	/// Handles a keyboard input.
	pub fn input(&mut self, key: KeyboardKey, action: KeyboardAction) {
		// TODO Write on /dev/input/event* files

		// TODO Handle several keyboards at a time
		match key {
			KeyboardKey::KeyLeftControl => self.ctrl = action == KeyboardAction::Pressed,
			KeyboardKey::KeyLeftShift => self.shift = action == KeyboardAction::Pressed,
			KeyboardKey::KeyLeftAlt => self.alt = action == KeyboardAction::Pressed,
			KeyboardKey::KeyRightAlt => self.right_alt = action == KeyboardAction::Pressed,
			KeyboardKey::KeyRightControl => self.right_ctrl = action == KeyboardAction::Pressed,

			_ => {},
		}

		if key == KeyboardKey::KeyNumberLock && self.number_lock.input(action) {
			self.set_led(KeyboardLED::NumberLock, self.number_lock.is_enabled());
		}
		if key == KeyboardKey::KeyCapsLock && self.caps_lock.input(action) {
			self.set_led(KeyboardLED::CapsLock, self.caps_lock.is_enabled());
		}
		if key == KeyboardKey::KeyScrollLock && self.scroll_lock.input(action) {
			self.set_led(KeyboardLED::ScrollLock, self.scroll_lock.is_enabled());
		}

		if action == KeyboardAction::Pressed {
			if self.ctrl && self.alt {
				match key {
					KeyboardKey::KeyF1 => tty::switch(0),
					KeyboardKey::KeyF2 => tty::switch(1),
					KeyboardKey::KeyF3 => tty::switch(2),
					KeyboardKey::KeyF4 => tty::switch(3),
					KeyboardKey::KeyF5 => tty::switch(4),
					KeyboardKey::KeyF6 => tty::switch(5),
					KeyboardKey::KeyF7 => tty::switch(6),
					KeyboardKey::KeyF8 => tty::switch(7),
					KeyboardKey::KeyF9 => tty::switch(8),
					KeyboardKey::KeyF10 => tty::switch(9),
					KeyboardKey::KeyF11 => tty::switch(10),
					KeyboardKey::KeyF12 => tty::switch(11),

					_ => {
					},
				}
			}

			let shift = self.shift != self.caps_lock.is_enabled();

			if let Some(tty_chars) = key.get_tty_chars(shift) {
				// TODO Write on TTY input
				crate::print!("{}", tty_chars); // TODO rm
			}
		}
	}

	/// Sets the state of the LED on every keyboards.
	/// `led` is the keyboard LED.
	/// `enabled` tells whether the LED is lit.
	pub fn set_led(&mut self, led: KeyboardLED, enabled: bool) {
		if let Some(ps2) = &mut self.ps2_keyboard {
			ps2.set_led(led, enabled);
		}
	}
}

impl DeviceManager for KeyboardManager {
	fn get_name(&self) -> &str {
		"kbd"
	}

	fn legacy_detect(&mut self) -> Result<(), Errno> {
		self.ps2_keyboard = Some(ps2::PS2Keyboard::new().or(Err(errno::EIO))?);
		Ok(())
	}

	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO (When plugging a keyboard, don't forget to set the LEDs state)
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) {
		// TODO
	}
}

impl Drop for KeyboardManager {
	fn drop(&mut self) {
		self.fini_device_files();
	}
}
