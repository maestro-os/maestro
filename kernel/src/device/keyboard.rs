/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Implementation of the keyboard device manager.

use crate::{
	device::manager::{DeviceManager, PhysicalDevice},
	tty,
};
use utils::errno::EResult;

/// Enumeration of keyboard keys.
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
}

impl KeyboardKey {
	// TODO Implement correctly with modifiers
	/// Returns the TTY characters for the given current.
	///
	/// Arguments:
	/// - `shift` tells whether shift is pressed. This value has to be inverted if
	/// caps lock is enabled.
	/// - `alt` tells whether alt is pressed.
	/// - `ctrl` tells whether control is pressed.
	/// - `meta` tells whether meta is pressed.
	pub fn get_tty_chars(
		&self,
		shift: bool,
		_alt: bool,
		ctrl: bool,
		_meta: bool,
	) -> Option<&[u8]> {
		match self {
			Self::KeyHome => return Some(b"\x1b[1~"),
			Self::KeyInsert => return Some(b"\x1b[2~"),
			Self::KeyDelete => return Some(b"\x1b[3~"),
			Self::KeyEnd => return Some(b"\x1b[4~"),
			Self::KeyPageUp => return Some(b"\x1b[5~"),
			Self::KeyPageDown => return Some(b"\x1b[6~"),
			Self::KeyF1 => return Some(b"\x1b[11~"),
			Self::KeyF2 => return Some(b"\x1b[12~"),
			Self::KeyF3 => return Some(b"\x1b[13~"),
			Self::KeyF4 => return Some(b"\x1b[14~"),
			Self::KeyF5 => return Some(b"\x1b[15~"),
			Self::KeyF6 => return Some(b"\x1b[17~"),
			Self::KeyF7 => return Some(b"\x1b[18~"),
			Self::KeyF8 => return Some(b"\x1b[19~"),
			Self::KeyF9 => return Some(b"\x1b[20~"),
			Self::KeyF10 => return Some(b"\x1b[21~"),
			Self::KeyF11 => return Some(b"\x1b[23~"),
			Self::KeyF12 => return Some(b"\x1b[24~"),
			_ => {}
		}

		if ctrl {
			match self {
				Self::KeyA => return Some(&[/* b'A' - b'A' + */ 1]),
				Self::KeyB => return Some(&[b'B' - b'A' + 1]),
				Self::KeyC => return Some(&[b'C' - b'A' + 1]),
				Self::KeyD => return Some(&[b'D' - b'A' + 1]),
				Self::KeyE => return Some(&[b'E' - b'A' + 1]),
				Self::KeyF => return Some(&[b'F' - b'A' + 1]),
				Self::KeyG => return Some(&[b'G' - b'A' + 1]),
				Self::KeyH => return Some(&[b'H' - b'A' + 1]),
				Self::KeyI => return Some(&[b'I' - b'A' + 1]),
				Self::KeyJ => return Some(&[b'J' - b'A' + 1]),
				Self::KeyK => return Some(&[b'K' - b'A' + 1]),
				Self::KeyL => return Some(&[b'L' - b'A' + 1]),
				Self::KeyM => return Some(&[b'M' - b'A' + 1]),
				Self::KeyN => return Some(&[b'N' - b'A' + 1]),
				Self::KeyO => return Some(&[b'O' - b'A' + 1]),
				Self::KeyP => return Some(&[b'P' - b'A' + 1]),
				Self::KeyQ => return Some(&[b'Q' - b'A' + 1]),
				Self::KeyR => return Some(&[b'R' - b'A' + 1]),
				Self::KeyS => return Some(&[b'S' - b'A' + 1]),
				Self::KeyT => return Some(&[b'T' - b'A' + 1]),
				Self::KeyU => return Some(&[b'U' - b'A' + 1]),
				Self::KeyV => return Some(&[b'V' - b'A' + 1]),
				Self::KeyW => return Some(&[b'W' - b'A' + 1]),
				Self::KeyX => return Some(&[b'X' - b'A' + 1]),
				Self::KeyY => return Some(&[b'Y' - b'A' + 1]),
				Self::KeyZ => return Some(&[b'Z' - b'A' + 1]),
				Self::KeyOpenBrace => return Some(&[b'[' - b'A' + 1]),
				Self::KeyBackslash => return Some(&[b'\\' - b'A' + 1]),
				Self::KeyCloseBrace => return Some(&[b']' - b'A' + 1]),

				Self::KeyCursorUp => return Some(b"\x1b[1;5A"),
				Self::KeyCursorLeft => return Some(b"\x1b[1;5D"),
				Self::KeyCursorRight => return Some(b"\x1b[1;5C"),
				Self::KeyCursorDown => return Some(b"\x1b[1;5B"),

				// TODO ^ and _
				_ => {}
			}
		}

		/*let mut modifier = 1;
		if shift {
			modifier += 1;
		}
		if alt {
			modifier += 2;
		}
		if ctrl {
			modifier += 4;
		}
		if meta {
			modifier += 8;
		}*/

		if !shift {
			match self {
				Self::KeyEsc => Some(b"\x1b"),
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
				Self::KeyBackspace => Some(b"\x7f"),
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

				Self::KeyKeypadEnter => Some(b"\n"),
				Self::KeyKeypadSlash => Some(b"/"),
				Self::KeyCursorUp => Some(b"\x1b[A"),
				Self::KeyCursorLeft => Some(b"\x1b[D"),
				Self::KeyCursorRight => Some(b"\x1b[C"),
				Self::KeyCursorDown => Some(b"\x1b[B"),

				_ => None,
			}
		} else {
			match self {
				Self::KeyEsc => Some(b"\x1b"),
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
				Self::KeyBackspace => Some(b"\x7f"),
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

				Self::KeyKeypadEnter => Some(b"\n"),
				Self::KeyKeypadSlash => Some(b"/"),
				// Self::KeyCursorUp => Some("\x1b[A"),
				// Self::KeyCursorLeft => Some("\x1b[C"),
				// Self::KeyCursorRight => Some("\x1b[D"),
				// Self::KeyCursorDown => Some("\x1b[B"),
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

/// A key that can enabled, such as caps lock.
///
/// Such a key is usually associated with an LED on the keyboard.
#[derive(Default)]
pub struct EnableKey {
	/// The key's state.
	state: bool,
	/// Tells whether to ignore the next pressed actions until a release action
	/// is received.
	///
	/// This allow to ignore repetitions.
	ignore: bool,
}

impl EnableKey {
	/// Handles a keyboard input.
	///
	/// `kbd_manager` is the keyboard manager.
	///
	/// If the state changed, the function returns `true`.
	pub fn input(&mut self, action: KeyboardAction) -> bool {
		match action {
			KeyboardAction::Pressed => {
				if !self.ignore {
					self.state = !self.state;
					self.ignore = true;
					return true;
				}
			}
			KeyboardAction::Released => {
				self.ignore = false;
			}
		}
		false
	}

	/// Tells whether the key is enabled.
	pub fn is_enabled(&self) -> bool {
		self.state
	}
}

/// Trait representing a physical keyboard.
pub trait Keyboard {
	/// Sets the state of the given LED.
	///
	/// Arguments:
	/// - `led` is the LED.
	/// - `enabled` tells whether the LED is enabled.
	fn set_led(&mut self, led: KeyboardLED, enabled: bool);
}

/// The keyboard manager structure.
pub struct KeyboardManager {
	/// The ctrl key state.
	ctrl: bool,
	/// The left shift key state.
	left_shift: bool,
	/// The right shift key state.
	right_shift: bool,
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
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		let s = Self {
			ctrl: false,
			left_shift: false,
			right_shift: false,
			alt: false,
			right_alt: false,
			right_ctrl: false,

			number_lock: EnableKey::default(),
			caps_lock: EnableKey::default(),
			scroll_lock: EnableKey::default(),
		};
		s.init_device_files();
		s
	}

	/// Initializes devices files.
	fn init_device_files(&self) {
		// TODO Create /dev/input/event* files
	}

	/// Destroys devices files.
	fn fini_device_files(&self) {
		// TODO Remove /dev/input/event* files
	}

	/// Handles a keyboard input.
	pub fn input(&mut self, key: KeyboardKey, action: KeyboardAction) {
		// TODO Write on /dev/input/event* files

		// TODO Handle several keyboards at a time
		match key {
			KeyboardKey::KeyLeftControl => self.ctrl = action == KeyboardAction::Pressed,
			KeyboardKey::KeyLeftShift => self.left_shift = action == KeyboardAction::Pressed,
			KeyboardKey::KeyRightShift => self.right_shift = action == KeyboardAction::Pressed,
			KeyboardKey::KeyLeftAlt => self.alt = action == KeyboardAction::Pressed,
			KeyboardKey::KeyRightAlt => self.right_alt = action == KeyboardAction::Pressed,
			KeyboardKey::KeyRightControl => self.right_ctrl = action == KeyboardAction::Pressed,

			_ => {}
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
				// FIXME: TTYs must be allocated first
				// Switch TTY
				let id = match key {
					KeyboardKey::KeyF1 => Some(0),
					KeyboardKey::KeyF2 => Some(1),
					KeyboardKey::KeyF3 => Some(2),
					KeyboardKey::KeyF4 => Some(3),
					KeyboardKey::KeyF5 => Some(4),
					KeyboardKey::KeyF6 => Some(5),
					KeyboardKey::KeyF7 => Some(6),
					KeyboardKey::KeyF8 => Some(7),
					KeyboardKey::KeyF9 => Some(8),
					_ => None,
				};
				tty::switch(id);
			}
			// Get tty
			if let Some(tty_mutex) = tty::current() {
				let mut tty = tty_mutex.lock();

				let ctrl = self.ctrl || self.right_ctrl;
				let alt = self.alt || self.right_alt;
				let shift = (self.left_shift || self.right_shift) != self.caps_lock.is_enabled();
				// TODO
				let meta = false;

				// Write on TTY
				if let Some(tty_chars) = key.get_tty_chars(shift, alt, ctrl, meta) {
					tty.input(tty_chars);
				}
			}
		}
	}

	/// Sets the state of the LED on every keyboards.
	///
	/// Arguments:
	/// - `led` is the keyboard LED.
	/// - `enabled` tells whether the LED is lit.
	pub fn set_led(&mut self, _led: KeyboardLED, _enabled: bool) {
		// TODO Iterate on keyboards
	}
}

impl DeviceManager for KeyboardManager {
	fn on_plug(&mut self, _dev: &dyn PhysicalDevice) -> EResult<()> {
		// TODO (When plugging a keyboard, don't forget to set the LEDs state)
		Ok(())
	}

	fn on_unplug(&mut self, _dev: &dyn PhysicalDevice) -> EResult<()> {
		// TODO
		Ok(())
	}
}

impl Drop for KeyboardManager {
	fn drop(&mut self) {
		self.fini_device_files();
	}
}
