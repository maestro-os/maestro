//! This module implements the keyboard device manager.

use crate::device::manager::DeviceManager;
use crate::device::manager::PhysicalDevice;
use crate::errno::Errno;
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
	/// Returns the TTY characters for the given current.
	/// `shift` tells whether shift is pressed. This value has to be inverted if caps lock is
	/// enabled.
	pub fn get_tty_chars(&self, shift: bool) -> Option<&[u8]> {
		if !shift {
			match self {
				Self::KeyEsc => Some(b"^["),
				Self::Key1 => Some(b"1"),
				Self::Key2 => Some(b"2"),
				Self::Key3 => Some(b"3"),
				Self::Key4 => Some(b"4"),
				Self::Key5 => Some(b"5"),
				Self::Key6 => Some(b"6"),
				Self::Key7 => Some(b"7"),
				Self::Key8 => Some(b"8"),
				Self::Key9 => Some(b"9"),
				Self::Key0 => Some(b"0"),
				Self::KeyMinus => Some(b"-"),
				Self::KeyEqual => Some(b"="),
				Self::KeyBackspace => Some(b"\x08"),
				Self::KeyTab => Some(b"\t"),
				Self::KeyQ => Some(b"q"),
				Self::KeyW => Some(b"w"),
				Self::KeyE => Some(b"e"),
				Self::KeyR => Some(b"r"),
				Self::KeyT => Some(b"t"),
				Self::KeyY => Some(b"y"),
				Self::KeyU => Some(b"u"),
				Self::KeyI => Some(b"i"),
				Self::KeyO => Some(b"o"),
				Self::KeyP => Some(b"p"),
				Self::KeyOpenBrace => Some(b"["),
				Self::KeyCloseBrace => Some(b"]"),
				Self::KeyEnter => Some(b"\n"),
				Self::KeyA => Some(b"a"),
				Self::KeyS => Some(b"s"),
				Self::KeyD => Some(b"d"),
				Self::KeyF => Some(b"f"),
				Self::KeyG => Some(b"g"),
				Self::KeyH => Some(b"h"),
				Self::KeyJ => Some(b"j"),
				Self::KeyK => Some(b"k"),
				Self::KeyL => Some(b"l"),
				Self::KeySemiColon => Some(b";"),
				Self::KeySingleQuote => Some(b"'"),
				Self::KeyBackTick => Some(b"`"),
				Self::KeyBackslash => Some(b"\\"),
				Self::KeyZ => Some(b"z"),
				Self::KeyX => Some(b"x"),
				Self::KeyC => Some(b"c"),
				Self::KeyV => Some(b"v"),
				Self::KeyB => Some(b"b"),
				Self::KeyN => Some(b"n"),
				Self::KeyM => Some(b"m"),
				Self::KeyComma => Some(b","),
				Self::KeyDot => Some(b"."),
				Self::KeySlash => Some(b"/"),
				Self::KeyKeypadStar => Some(b"*"),
				Self::KeySpace => Some(b" "),
				Self::KeyF1 => Some(b"^[[[A"),
				Self::KeyF2 => Some(b"^[[[B"),
				Self::KeyF3 => Some(b"^[[[C"),
				Self::KeyF4 => Some(b"^[[[D"),
				Self::KeyF5 => Some(b"^[[[E"),
				Self::KeyF6 => Some(b"^[[17"),
				Self::KeyF7 => Some(b"^[[18"),
				Self::KeyF8 => Some(b"^[[19"),
				Self::KeyF9 => Some(b"^[[20"),
				Self::KeyF10 => Some(b"^[[21"),
				Self::KeyKeypad7 => Some(b"7"),
				Self::KeyKeypad8 => Some(b"8"),
				Self::KeyKeypad9 => Some(b"9"),
				Self::KeyKeypadMinus => Some(b"-"),
				Self::KeyKeypad4 => Some(b"4"),
				Self::KeyKeypad5 => Some(b"5"),
				Self::KeyKeypad6 => Some(b"6"),
				Self::KeyKeypadPlus => Some(b"+"),
				Self::KeyKeypad1 => Some(b"1"),
				Self::KeyKeypad2 => Some(b"2"),
				Self::KeyKeypad3 => Some(b"3"),
				Self::KeyKeypad0 => Some(b"0"),
				Self::KeyKeypadDot => Some(b"."),
				Self::KeyF11 => Some(b"^[[23~"),
				Self::KeyF12 => Some(b"^[[24~"),

				Self::KeyKeypadEnter => Some(b"\n"),
				Self::KeyKeypadSlash => Some(b"/"),
				Self::KeyHome => Some(b"^[[1~"),
				Self::KeyCursorUp => Some(b"^[[A"),
				Self::KeyPageUp => Some(b"^[[5~"),
				Self::KeyCursorLeft => Some(b"^[[C"),
				Self::KeyCursorRight => Some(b"^[[D"),
				Self::KeyEnd => Some(b"^[[4~"),
				Self::KeyCursorDown => Some(b"^[[B"),
				Self::KeyPageDown => Some(b"^[[6~"),

				_ => None,
			}
		} else {
			match self {
				Self::KeyEsc => Some(b"^["),
				Self::Key1 => Some(b"!"),
				Self::Key2 => Some(b"@"),
				Self::Key3 => Some(b"#"),
				Self::Key4 => Some(b"$"),
				Self::Key5 => Some(b"%"),
				Self::Key6 => Some(b"^"),
				Self::Key7 => Some(b"&"),
				Self::Key8 => Some(b"*"),
				Self::Key9 => Some(b"("),
				Self::Key0 => Some(b")"),
				Self::KeyMinus => Some(b"_"),
				Self::KeyEqual => Some(b"+"),
				Self::KeyBackspace => Some(b"\x08"),
				Self::KeyTab => Some(b"\t"),
				Self::KeyQ => Some(b"Q"),
				Self::KeyW => Some(b"W"),
				Self::KeyE => Some(b"E"),
				Self::KeyR => Some(b"R"),
				Self::KeyT => Some(b"T"),
				Self::KeyY => Some(b"Y"),
				Self::KeyU => Some(b"U"),
				Self::KeyI => Some(b"I"),
				Self::KeyO => Some(b"O"),
				Self::KeyP => Some(b"P"),
				Self::KeyOpenBrace => Some(b"{"),
				Self::KeyCloseBrace => Some(b"}"),
				Self::KeyEnter => Some(b"\n"),
				Self::KeyA => Some(b"A"),
				Self::KeyS => Some(b"S"),
				Self::KeyD => Some(b"D"),
				Self::KeyF => Some(b"F"),
				Self::KeyG => Some(b"G"),
				Self::KeyH => Some(b"H"),
				Self::KeyJ => Some(b"J"),
				Self::KeyK => Some(b"K"),
				Self::KeyL => Some(b"L"),
				Self::KeySemiColon => Some(b":"),
				Self::KeySingleQuote => Some(b"\""),
				Self::KeyBackTick => Some(b"~"),
				Self::KeyBackslash => Some(b"|"),
				Self::KeyZ => Some(b"Z"),
				Self::KeyX => Some(b"X"),
				Self::KeyC => Some(b"C"),
				Self::KeyV => Some(b"V"),
				Self::KeyB => Some(b"B"),
				Self::KeyN => Some(b"N"),
				Self::KeyM => Some(b"M"),
				Self::KeyComma => Some(b"<"),
				Self::KeyDot => Some(b">"),
				Self::KeySlash => Some(b"?"),
				Self::KeyKeypadStar => Some(b"*"),
				Self::KeySpace => Some(b" "),
				// TODO F1 to F10
				Self::KeyKeypad7 => Some(b"7"),
				Self::KeyKeypad8 => Some(b"8"),
				Self::KeyKeypad9 => Some(b"9"),
				Self::KeyKeypadMinus => Some(b"-"),
				Self::KeyKeypad4 => Some(b"4"),
				Self::KeyKeypad5 => Some(b"5"),
				Self::KeyKeypad6 => Some(b"6"),
				Self::KeyKeypadPlus => Some(b"+"),
				Self::KeyKeypad1 => Some(b"1"),
				Self::KeyKeypad2 => Some(b"2"),
				Self::KeyKeypad3 => Some(b"3"),
				Self::KeyKeypad0 => Some(b"0"),
				Self::KeyKeypadDot => Some(b"."),
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
				// Switching TTY
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

			// Getting the tty
			let mut tty_guard = tty::current().lock();

			if key == KeyboardKey::KeyBackspace {
				// Erasing from TTY
				tty_guard.get_mut().erase(1);
			} else {
				// Writing on TTY
				let shift = self.shift != self.caps_lock.is_enabled();

				if let Some(tty_chars) = key.get_tty_chars(shift) {
					tty_guard.get_mut().input(tty_chars);
				}
			}
		}
	}

	/// Sets the state of the LED on every keyboards.
	/// `led` is the keyboard LED.
	/// `enabled` tells whether the LED is lit.
	pub fn set_led(&mut self, _led: KeyboardLED, _enabled: bool) {
		// TODO Iterate on keyboards
		/*if let Some(ps2) = &mut self.ps2_keyboard {
			ps2.set_led(led, enabled);
		}*/

		todo!();
	}
}

impl DeviceManager for KeyboardManager {
	fn get_name(&self) -> &str {
		"kbd"
	}

	fn legacy_detect(&mut self) -> Result<(), Errno> {
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
