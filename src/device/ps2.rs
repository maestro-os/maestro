//! Personal System/2 (PS/2) is a connector designed for keyboards and mouses.
//! It has now been deprecated in favor of USB keyboards/mouses.

// TODO Externalize this module into a kernel module when the interface for loading them will be
// ready

use crate::event::{CallbackHook, InterruptResult, InterruptResultAction};
use crate::event;
use crate::io;
use crate::module::Module;
use crate::module;
use crate::util::boxed::Box;
use crate::util;

/// The interrupt number for keyboard input events.
const KEYBOARD_INTERRUPT_ID: usize = 33;

/// TODO doc
const DATA_REGISTER: u16 = 0x60;
/// TODO doc
const STATUS_REGISTER: u16 = 0x64;
/// TODO doc
const COMMAND_REGISTER: u16 = 0x64;

/// The maximum number of attempts for sending a command to the PS/2 controller.
const MAX_ATTEMPTS: usize = 3;

/// TODO doc
const TEST_CONTROLLER_PASS: u8 = 0x55;
/// TODO doc
const TEST_CONTROLLER_FAIL: u8 = 0xfc;

/// TODO doc
const TEST_KEYBOARD_PASS: u8 = 0x00;
// TODO TEST_KEYBOARD_FAIL

/// TODO doc
const KEYBOARD_ACK: u8 = 0xfa;
/// TODO doc
const KEYBOARD_RESEND: u8 = 0xf4;

/// The ID of the Scroll Lock LED.
const LED_SCROLL_LOCK: u8 = 0b001;
/// The ID of the Number Lock LED.
const LED_NUMBER_LOCK: u8 = 0b010;
/// The ID of the Caps Lock LED.
const LED_CAPS_LOCK: u8 = 0b100;

// TODO Turn commands and flags into constants.

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

/// Enumeration of keyboard actions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyboardAction {
	/// The key was pressed.
	Pressed,
	/// The key was released.
	Released,
}

/// A slice containing a pair of keycode and enumeration that allows to associate a keycode with
/// its enumeration entry.
static NORMAL_KEYS: [(u8, KeyboardKey); 85] = [
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

/// Same as `NORMAL_KEYS` except this slice stores keys beginning with `0xE0`.
static SPECIAL_KEYS: [(u8, KeyboardKey); 38] = [
	(0x10, KeyboardKey::KeyPreviousTrack),
	(0x19, KeyboardKey::KeyNextTrack),
	(0x1C, KeyboardKey::KeyKeypadEnter),
	(0x1D, KeyboardKey::KeyRightControl),
	(0x20, KeyboardKey::KeyMute),
	(0x21, KeyboardKey::KeyCalculator),
	(0x22, KeyboardKey::KeyPlay),
	(0x24, KeyboardKey::KeyStop),
	(0x2E, KeyboardKey::KeyVolumeDown),
	(0x30, KeyboardKey::KeyVolumeUp),
	(0x32, KeyboardKey::KeyWWWHome),
	(0x35, KeyboardKey::KeyKeypadSlash),
	(0x38, KeyboardKey::KeyRightAlt),
	(0x47, KeyboardKey::KeyHome),
	(0x48, KeyboardKey::KeyCursorUp),
	(0x49, KeyboardKey::KeyPageUp),
	(0x4B, KeyboardKey::KeyCursorLeft),
	(0x4D, KeyboardKey::KeyCursorRight),
	(0x4F, KeyboardKey::KeyEnd),
	(0x50, KeyboardKey::KeyCursorDown),
	(0x51, KeyboardKey::KeyPageDown),
	(0x52, KeyboardKey::KeyInsert),
	(0x53, KeyboardKey::KeyDelete),
	(0x5B, KeyboardKey::KeyLeftGUI),
	(0x5C, KeyboardKey::KeyRightGUI),
	(0x5D, KeyboardKey::KeyApps),
	(0x5E, KeyboardKey::KeyACPIPower),
	(0x5F, KeyboardKey::KeyACPISleep),
	(0x63, KeyboardKey::KeyACPIWake),
	(0x65, KeyboardKey::KeyWWWSearch),
	(0x66, KeyboardKey::KeyWWWFavorites),
	(0x67, KeyboardKey::KeyWWWRefresh),
	(0x68, KeyboardKey::KeyWWWStop),
	(0x69, KeyboardKey::KeyWWWForward),
	(0x6A, KeyboardKey::KeyWWWBack),
	(0x6B, KeyboardKey::KeyMyComputer),
	(0x6C, KeyboardKey::KeyEmail),
	(0x6D, KeyboardKey::KeyMediaSelect),
];

/// Tells whether the PS/2 buffer is ready for reading.
fn can_read() -> bool {
	unsafe {
		io::inb(STATUS_REGISTER) & 0b1 != 0
	}
}

/// Tells whether the PS/2 buffer is ready for writing.
fn can_write() -> bool {
	unsafe {
		io::inb(STATUS_REGISTER) & 0b10 == 0
	}
}

/// Waits until the buffer is ready for reading.
fn wait_read() {
	while !can_read() {}
}

/// Waits until the buffer is ready for reading.
fn wait_write() {
	while !can_write() {}
}

/// Clears the PS/2 controller's buffer.
fn clear_buffer() {
	while can_read() {
		unsafe {
			io::inb(DATA_REGISTER);
		}
	}
}

/// Sends the given data `data` to the keyboard.
fn keyboard_send(data: u8) -> Result<(), ()> {
	let mut response = 0;

	for _ in 0..MAX_ATTEMPTS {
		wait_write();
		unsafe {
			io::outb(DATA_REGISTER, data);
		}

		wait_read();
		response = unsafe {
			io::inb(DATA_REGISTER)
		};
		if response == KEYBOARD_ACK {
			return Ok(());
		}
	}

	if response == KEYBOARD_ACK {
		Ok(())
	} else {
		Err(())
	}
}

/// Sends the given command `command` and returns the response.
fn send_command(command: u8, expected_response: u8) -> Result<(), ()> {
	for _ in 0..MAX_ATTEMPTS {
		wait_write();
		unsafe {
			io::outb(COMMAND_REGISTER, command);
		}

		wait_read();
		let response = unsafe {
			io::inb(DATA_REGISTER)
		};
		if response == expected_response {
			return Ok(());
		}
	}
	Err(())
}

/// Disables PS/2 devices.
fn disable_devices() {
	wait_write();
	unsafe {
		io::outb(COMMAND_REGISTER, 0xad);
	}

	wait_write();
	unsafe {
		io::outb(COMMAND_REGISTER, 0xa7);
	}
}

/// Enables the keyboard device.
fn enable_keyboard() -> Result<(), ()> {
	wait_write();
	unsafe {
		io::outb(COMMAND_REGISTER, 0xae);
	}

	keyboard_send(0xf0)?;
	keyboard_send(1)?;
	keyboard_send(0xf3)?;
	keyboard_send(0)?;
	keyboard_send(0xf4)?;
	Ok(())
}

/// Returns the configuration byte.
fn get_config_byte() -> u8 {
	wait_write();
	unsafe {
		io::outb(COMMAND_REGISTER, 0x20);
	}

	wait_read();
	unsafe {
		io::inb(DATA_REGISTER)
	}
}

/// Sets the configuration byte.
fn set_config_byte(config: u8) {
	wait_write();
	unsafe {
		io::outb(COMMAND_REGISTER, 0x60);
	}

	wait_write();
	unsafe {
		io::outb(DATA_REGISTER, config);
	}
}

/// Tests the PS/2 controller.
fn test_controller() -> Result<(), ()> {
	send_command(0xaa, TEST_CONTROLLER_PASS)
}

/// Tests the keyboard device.
fn test_device() -> Result<(), ()> {
	send_command(0xab, TEST_KEYBOARD_PASS)
}

/// Reads one byte of keycode from the controller.
fn read_keycode_byte() -> u8 {
	unsafe {
		io::inb(DATA_REGISTER)
	}
}

/// Reads a keystroke and returns the associated key and action.
fn read_keystroke() -> (KeyboardKey, KeyboardAction) {
	let mut keycode = read_keycode_byte();
	let special = keycode == 0xe0;
	if special {
		keycode = read_keycode_byte();
	}
	// TODO Add support for print screen and pause

	let action = if keycode < 0x80 {
		KeyboardAction::Pressed
	} else {
		keycode -= 0x80;
		KeyboardAction::Released
	};

	let cmp = | k: &(u8, KeyboardKey) | {
		k.0.cmp(&keycode)
	};
	let list = if !special {
		&NORMAL_KEYS[..]
	} else {
		&SPECIAL_KEYS[..]
	};
	let index = list.binary_search_by(cmp);
	let key = if let Ok(i) = index {
		list[i].1
	} else {
		KeyboardKey::KeyUnknown
	};

	(key, action)
}

/// The PS2 kernel module structure.
pub struct PS2Module {
	/// The callback hook for keyboard input interrupts.
	keyboard_interrupt_callback_hook: Option<CallbackHook>,
	/// The callback handling keyboard inputs.
	keyboard_callback: Box<dyn FnMut(KeyboardKey, KeyboardAction)>,
}

impl PS2Module {
	/// Creates the module's instance.
	pub fn new<F: 'static + FnMut(KeyboardKey, KeyboardAction)>(f: F) -> Self {
		Self {
			keyboard_interrupt_callback_hook: None,
			keyboard_callback: Box::new(f).unwrap(),
		}
	}
}

impl Module for PS2Module {
	fn get_name(&self) -> &str {
		"PS/2"
	}

	fn get_version(&self) -> module::Version {
		module::Version {
			major: 0,
			minor: 0,
			patch: 0,
		}
	}

	fn init(&mut self) -> Result<(), ()> {
		// TODO Check if PS/2 controller is existing using ACPI

		// TODO Disable interrupts during init

		disable_devices();
		clear_buffer();

		set_config_byte(get_config_byte() & 0b10111100);

		test_controller()?;
		test_device()?;
		enable_keyboard()?;

		set_config_byte(get_config_byte() | 0b1);
		clear_buffer();

		let callback = | _id: u32, _code: u32, _regs: &util::Regs, _ring: u32 | {
			while can_read() {
				let (key, action) = read_keystroke();

				// TODO Do something with the key

				if key == KeyboardKey::KeyPause && action == KeyboardAction::Pressed {
					// TODO Release the key
				}
			}

			InterruptResult::new(false, InterruptResultAction::Resume)
		};
		let hook_result = event::register_callback(KEYBOARD_INTERRUPT_ID, 0, callback);
		if let Ok(hook) = hook_result {
			self.keyboard_interrupt_callback_hook = Some(hook);
			Ok(())
		} else {
			Err(())
		}
	}

	// TODO LEDs state
}
