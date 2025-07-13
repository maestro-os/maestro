//! Handles scancode sets and keycodes decoding.

use crate::{KBD_CMD_SCANCODE, keyboard_send, read_data};
use kernel::device::keyboard::{KeyboardAction, KeyboardKey};

static SET1_BASE_KEYS: [(u8, KeyboardKey); 85] = [
	(0x01, KeyboardKey::KeyEsc),
	(0x02, KeyboardKey::Key1),
	(0x03, KeyboardKey::Key2),
	(0x04, KeyboardKey::Key3),
	(0x05, KeyboardKey::Key4),
	(0x06, KeyboardKey::Key5),
	(0x07, KeyboardKey::Key6),
	(0x08, KeyboardKey::Key7),
	(0x09, KeyboardKey::Key8),
	(0x0a, KeyboardKey::Key9),
	(0x0b, KeyboardKey::Key0),
	(0x0c, KeyboardKey::KeyMinus),
	(0x0d, KeyboardKey::KeyEqual),
	(0x0e, KeyboardKey::KeyBackspace),
	(0x0f, KeyboardKey::KeyTab),
	(0x10, KeyboardKey::KeyQ),
	(0x11, KeyboardKey::KeyW),
	(0x12, KeyboardKey::KeyE),
	(0x13, KeyboardKey::KeyR),
	(0x14, KeyboardKey::KeyT),
	(0x15, KeyboardKey::KeyY),
	(0x16, KeyboardKey::KeyU),
	(0x17, KeyboardKey::KeyI),
	(0x18, KeyboardKey::KeyO),
	(0x19, KeyboardKey::KeyP),
	(0x1a, KeyboardKey::KeyOpenBrace),
	(0x1b, KeyboardKey::KeyCloseBrace),
	(0x1c, KeyboardKey::KeyEnter),
	(0x1d, KeyboardKey::KeyLeftControl),
	(0x1e, KeyboardKey::KeyA),
	(0x1f, KeyboardKey::KeyS),
	(0x20, KeyboardKey::KeyD),
	(0x21, KeyboardKey::KeyF),
	(0x22, KeyboardKey::KeyG),
	(0x23, KeyboardKey::KeyH),
	(0x24, KeyboardKey::KeyJ),
	(0x25, KeyboardKey::KeyK),
	(0x26, KeyboardKey::KeyL),
	(0x27, KeyboardKey::KeySemiColon),
	(0x28, KeyboardKey::KeySingleQuote),
	(0x29, KeyboardKey::KeyBackTick),
	(0x2a, KeyboardKey::KeyLeftShift),
	(0x2b, KeyboardKey::KeyBackslash),
	(0x2c, KeyboardKey::KeyZ),
	(0x2d, KeyboardKey::KeyX),
	(0x2e, KeyboardKey::KeyC),
	(0x2f, KeyboardKey::KeyV),
	(0x30, KeyboardKey::KeyB),
	(0x31, KeyboardKey::KeyN),
	(0x32, KeyboardKey::KeyM),
	(0x33, KeyboardKey::KeyComma),
	(0x34, KeyboardKey::KeyDot),
	(0x35, KeyboardKey::KeySlash),
	(0x36, KeyboardKey::KeyRightShift),
	(0x37, KeyboardKey::KeyKeypadStar),
	(0x38, KeyboardKey::KeyLeftAlt),
	(0x39, KeyboardKey::KeySpace),
	(0x3a, KeyboardKey::KeyCapsLock),
	(0x3b, KeyboardKey::KeyF1),
	(0x3c, KeyboardKey::KeyF2),
	(0x3d, KeyboardKey::KeyF3),
	(0x3e, KeyboardKey::KeyF4),
	(0x3f, KeyboardKey::KeyF5),
	(0x40, KeyboardKey::KeyF6),
	(0x41, KeyboardKey::KeyF7),
	(0x42, KeyboardKey::KeyF8),
	(0x43, KeyboardKey::KeyF9),
	(0x44, KeyboardKey::KeyF10),
	(0x45, KeyboardKey::KeyNumberLock),
	(0x46, KeyboardKey::KeyScrollLock),
	(0x47, KeyboardKey::KeyKeypad7),
	(0x48, KeyboardKey::KeyKeypad8),
	(0x49, KeyboardKey::KeyKeypad9),
	(0x4a, KeyboardKey::KeyKeypadMinus),
	(0x4b, KeyboardKey::KeyKeypad4),
	(0x4c, KeyboardKey::KeyKeypad5),
	(0x4d, KeyboardKey::KeyKeypad6),
	(0x4e, KeyboardKey::KeyKeypadPlus),
	(0x4f, KeyboardKey::KeyKeypad1),
	(0x50, KeyboardKey::KeyKeypad2),
	(0x51, KeyboardKey::KeyKeypad3),
	(0x52, KeyboardKey::KeyKeypad0),
	(0x53, KeyboardKey::KeyKeypadDot),
	(0x57, KeyboardKey::KeyF11),
	(0x58, KeyboardKey::KeyF12),
];

static SET1_SPECIAL_KEYS: [(u8, KeyboardKey); 38] = [
	(0x10, KeyboardKey::KeyPreviousTrack),
	(0x19, KeyboardKey::KeyNextTrack),
	(0x1c, KeyboardKey::KeyKeypadEnter),
	(0x1d, KeyboardKey::KeyRightControl),
	(0x20, KeyboardKey::KeyMute),
	(0x21, KeyboardKey::KeyCalculator),
	(0x22, KeyboardKey::KeyPlay),
	(0x24, KeyboardKey::KeyStop),
	(0x2e, KeyboardKey::KeyVolumeDown),
	(0x30, KeyboardKey::KeyVolumeUp),
	(0x32, KeyboardKey::KeyWWWHome),
	(0x35, KeyboardKey::KeyKeypadSlash),
	(0x38, KeyboardKey::KeyRightAlt),
	(0x47, KeyboardKey::KeyHome),
	(0x48, KeyboardKey::KeyCursorUp),
	(0x49, KeyboardKey::KeyPageUp),
	(0x4b, KeyboardKey::KeyCursorLeft),
	(0x4d, KeyboardKey::KeyCursorRight),
	(0x4f, KeyboardKey::KeyEnd),
	(0x50, KeyboardKey::KeyCursorDown),
	(0x51, KeyboardKey::KeyPageDown),
	(0x52, KeyboardKey::KeyInsert),
	(0x53, KeyboardKey::KeyDelete),
	(0x5b, KeyboardKey::KeyLeftGUI),
	(0x5c, KeyboardKey::KeyRightGUI),
	(0x5d, KeyboardKey::KeyApps),
	(0x5e, KeyboardKey::KeyACPIPower),
	(0x5f, KeyboardKey::KeyACPISleep),
	(0x63, KeyboardKey::KeyACPIWake),
	(0x65, KeyboardKey::KeyWWWSearch),
	(0x66, KeyboardKey::KeyWWWFavorites),
	(0x67, KeyboardKey::KeyWWWRefresh),
	(0x68, KeyboardKey::KeyWWWStop),
	(0x69, KeyboardKey::KeyWWWForward),
	(0x6a, KeyboardKey::KeyWWWBack),
	(0x6b, KeyboardKey::KeyMyComputer),
	(0x6c, KeyboardKey::KeyEmail),
	(0x6d, KeyboardKey::KeyMediaSelect),
];

static SET2_BASE_KEYS: [(u8, KeyboardKey); 85] = [
	(0x01, KeyboardKey::KeyF9),
	(0x03, KeyboardKey::KeyF5),
	(0x04, KeyboardKey::KeyF3),
	(0x05, KeyboardKey::KeyF1),
	(0x06, KeyboardKey::KeyF2),
	(0x07, KeyboardKey::KeyF12),
	(0x09, KeyboardKey::KeyF10),
	(0x0a, KeyboardKey::KeyF8),
	(0x0b, KeyboardKey::KeyF6),
	(0x0c, KeyboardKey::KeyF4),
	(0x0d, KeyboardKey::KeyTab),
	(0x0e, KeyboardKey::KeyBackTick),
	(0x11, KeyboardKey::KeyLeftAlt),
	(0x12, KeyboardKey::KeyLeftShift),
	(0x14, KeyboardKey::KeyLeftControl),
	(0x15, KeyboardKey::KeyQ),
	(0x16, KeyboardKey::Key1),
	(0x1a, KeyboardKey::KeyZ),
	(0x1b, KeyboardKey::KeyS),
	(0x1c, KeyboardKey::KeyA),
	(0x1d, KeyboardKey::KeyW),
	(0x1e, KeyboardKey::Key2),
	(0x21, KeyboardKey::KeyC),
	(0x22, KeyboardKey::KeyX),
	(0x23, KeyboardKey::KeyD),
	(0x24, KeyboardKey::KeyE),
	(0x25, KeyboardKey::Key4),
	(0x26, KeyboardKey::Key3),
	(0x29, KeyboardKey::KeySpace),
	(0x2a, KeyboardKey::KeyV),
	(0x2b, KeyboardKey::KeyF),
	(0x2c, KeyboardKey::KeyT),
	(0x2d, KeyboardKey::KeyR),
	(0x2e, KeyboardKey::Key5),
	(0x31, KeyboardKey::KeyN),
	(0x32, KeyboardKey::KeyB),
	(0x33, KeyboardKey::KeyH),
	(0x34, KeyboardKey::KeyG),
	(0x35, KeyboardKey::KeyY),
	(0x36, KeyboardKey::Key6),
	(0x3a, KeyboardKey::KeyM),
	(0x3b, KeyboardKey::KeyJ),
	(0x3c, KeyboardKey::KeyU),
	(0x3d, KeyboardKey::Key7),
	(0x3e, KeyboardKey::Key8),
	(0x41, KeyboardKey::KeyComma),
	(0x42, KeyboardKey::KeyK),
	(0x43, KeyboardKey::KeyI),
	(0x44, KeyboardKey::KeyO),
	(0x45, KeyboardKey::Key0),
	(0x46, KeyboardKey::Key9),
	(0x49, KeyboardKey::KeyDot),
	(0x4a, KeyboardKey::KeySlash),
	(0x4b, KeyboardKey::KeyL),
	(0x4c, KeyboardKey::KeySemiColon),
	(0x4d, KeyboardKey::KeyP),
	(0x4e, KeyboardKey::KeyMinus),
	(0x52, KeyboardKey::KeySingleQuote),
	(0x54, KeyboardKey::KeyOpenBrace),
	(0x55, KeyboardKey::KeyEqual),
	(0x58, KeyboardKey::KeyCapsLock),
	(0x59, KeyboardKey::KeyRightShift),
	(0x5a, KeyboardKey::KeyEnter),
	(0x5b, KeyboardKey::KeyCloseBrace),
	(0x5d, KeyboardKey::KeySlash),
	(0x66, KeyboardKey::KeyBackspace),
	(0x69, KeyboardKey::KeyKeypad1),
	(0x6b, KeyboardKey::KeyKeypad4),
	(0x6c, KeyboardKey::KeyKeypad7),
	(0x70, KeyboardKey::KeyKeypad0),
	(0x71, KeyboardKey::KeyKeypadDot),
	(0x72, KeyboardKey::KeyKeypad2),
	(0x73, KeyboardKey::KeyKeypad5),
	(0x74, KeyboardKey::KeyKeypad6),
	(0x75, KeyboardKey::KeyKeypad8),
	(0x76, KeyboardKey::KeyEsc),
	(0x77, KeyboardKey::KeyNumberLock),
	(0x78, KeyboardKey::KeyF11),
	(0x79, KeyboardKey::KeyKeypadPlus),
	(0x7a, KeyboardKey::KeyKeypad3),
	(0x7b, KeyboardKey::KeyKeypadMinus),
	(0x7c, KeyboardKey::KeyKeypadStar),
	(0x7d, KeyboardKey::KeyKeypad9),
	(0x7e, KeyboardKey::KeyScrollLock),
	(0x83, KeyboardKey::KeyF7),
];

static SET2_SPECIAL_KEYS: [(u8, KeyboardKey); 38] = [
	(0x10, KeyboardKey::KeyWWWSearch),
	(0x11, KeyboardKey::KeyRightAlt),
	(0x14, KeyboardKey::KeyRightControl),
	(0x15, KeyboardKey::KeyPreviousTrack),
	(0x18, KeyboardKey::KeyWWWFavorites),
	(0x1f, KeyboardKey::KeyLeftGUI),
	(0x20, KeyboardKey::KeyWWWRefresh),
	(0x21, KeyboardKey::KeyVolumeDown),
	(0x23, KeyboardKey::KeyMute),
	(0x27, KeyboardKey::KeyRightGUI),
	(0x28, KeyboardKey::KeyWWWStop),
	(0x2b, KeyboardKey::KeyCalculator),
	(0x2f, KeyboardKey::KeyApps),
	(0x30, KeyboardKey::KeyWWWForward),
	(0x32, KeyboardKey::KeyVolumeUp),
	(0x34, KeyboardKey::KeyPlay),
	(0x37, KeyboardKey::KeyACPIPower),
	(0x38, KeyboardKey::KeyWWWBack),
	(0x3a, KeyboardKey::KeyWWWHome),
	(0x3b, KeyboardKey::KeyStop),
	(0x3f, KeyboardKey::KeyACPISleep),
	(0x40, KeyboardKey::KeyMyComputer),
	(0x48, KeyboardKey::KeyEmail),
	(0x4a, KeyboardKey::KeyKeypadSlash),
	(0x4d, KeyboardKey::KeyNextTrack),
	(0x50, KeyboardKey::KeyMediaSelect),
	(0x5a, KeyboardKey::KeyEnter),
	(0x5e, KeyboardKey::KeyACPIWake),
	(0x69, KeyboardKey::KeyEnd),
	(0x6b, KeyboardKey::KeyCursorLeft),
	(0x6c, KeyboardKey::KeyHome),
	(0x70, KeyboardKey::KeyInsert),
	(0x71, KeyboardKey::KeyDelete),
	(0x72, KeyboardKey::KeyCursorDown),
	(0x74, KeyboardKey::KeyCursorRight),
	(0x75, KeyboardKey::KeyCursorUp),
	(0x7a, KeyboardKey::KeyPageDown),
	(0x7d, KeyboardKey::KeyPageUp),
];

/// Enumeration of scancode sets.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ScancodeSet {
	Set1,
	Set2,
	Set3,
}

impl TryFrom<u8> for ScancodeSet {
	type Error = ();

	/// Returns the scancode set corresponding to the given ID.
	fn try_from(n: u8) -> Result<Self, ()> {
		match n {
			1 | 0x43 => Ok(Self::Set1),
			2 | 0x41 => Ok(Self::Set2),
			3 | 0x3f => Ok(Self::Set3),
			_ => Err(()),
		}
	}
}

impl TryInto<u8> for ScancodeSet {
	type Error = ();

	fn try_into(self) -> Result<u8, Self::Error> {
		match self {
			Self::Set1 => Ok(1),
			Self::Set2 => Ok(2),
			Self::Set3 => Ok(3),
		}
	}
}

impl ScancodeSet {
	/// Returns the current scancode set.
	pub fn current() -> Result<Self, ()> {
		// Get current scancode set
		keyboard_send(KBD_CMD_SCANCODE)?;
		keyboard_send(0)?;
		let n = read_data();
		// Translate
		Self::try_from(n)
	}

	/// Sets the scancode as current.
	pub fn set_current(self) -> Result<(), ()> {
		keyboard_send(KBD_CMD_SCANCODE)?;
		keyboard_send(self.try_into()?)
	}

	/// Fallbacks to the next set if necessary.
	///
	/// If no supported set can be used, the function returns an error.
	#[allow(unused)]
	pub fn fallback(self) -> Result<Self, ()> {
		if self != Self::Set3 {
			return Ok(self);
		}
		let mut set = Self::Set2;
		loop {
			// Test
			set.set_current()?;
			let cur = Self::current()?;
			if cur == set {
				return Ok(set);
			}
			// Try next
			set = match set {
				Self::Set2 => Self::Set1,
				_ => break,
			};
		}
		Err(())
	}

	/// Reads a keystroke and returns the associated key and action.
	pub fn read_keystroke(&self) -> Option<(KeyboardKey, KeyboardAction)> {
		let mut keycode = read_data();
		let special = keycode == 0xe0;
		if special {
			keycode = read_data();
		}
		let action = match self {
			Self::Set1 => {
				if keycode < 0x80 {
					KeyboardAction::Pressed
				} else {
					keycode -= 0x80;
					KeyboardAction::Released
				}
			}
			Self::Set2 => {
				if keycode == 0xf0 {
					keycode = read_data();
					KeyboardAction::Released
				} else {
					KeyboardAction::Pressed
				}
			}
			_ => return None,
		};
		// TODO Add support for print screen and pause

		let codes = match (self, special) {
			(Self::Set1, false) => &SET1_BASE_KEYS[..],
			(Self::Set1, true) => &SET1_SPECIAL_KEYS[..],
			(Self::Set2, false) => &SET2_BASE_KEYS[..],
			(Self::Set2, true) => &SET2_SPECIAL_KEYS[..],
			// Checked earlier
			_ => unreachable!(),
		};
		if let Ok(i) = codes.binary_search_by(|k: &(u8, KeyboardKey)| k.0.cmp(&keycode)) {
			Some((codes[i].1, action))
		} else {
			None
		}
	}
}
