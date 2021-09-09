//! Personal System/2 (PS/2) is a connector designed for keyboards and mouses.
//! It has now been deprecated in favor of USB keyboards/mouses.

// TODO Externalize this into a kernel module when the interface for loading them will be ready

use crate::device::DeviceManager;
use crate::device::keyboard::Keyboard;
use crate::device::keyboard::KeyboardAction;
use crate::device::keyboard::KeyboardKey;
use crate::device::keyboard::KeyboardLED;
use crate::device::keyboard::KeyboardManager;
use crate::device::manager;
use crate::event::{CallbackHook, InterruptResult, InterruptResultAction};
use crate::event;
use crate::idt;
use crate::io;
use crate::util;

/// The interrupt number for keyboard input events.
const KEYBOARD_INTERRUPT_ID: usize = 33;

/// The PS/2 controller data port.
const DATA_REGISTER: u16 = 0x60;
/// The PS/2 controller status port.
const STATUS_REGISTER: u16 = 0x64;
/// The PS/2 controller status port.
const COMMAND_REGISTER: u16 = 0x64;

/// The maximum number of attempts for sending a command to the PS/2 controller.
const MAX_ATTEMPTS: usize = 3;

/// Response telling the test passed.
const TEST_CONTROLLER_PASS: u8 = 0x55;
/// Response telling the test failed.
const TEST_CONTROLLER_FAIL: u8 = 0xfc;

/// Response telling the keyboard test passed.
const TEST_KEYBOARD_PASS: u8 = 0x00;

/// Command to set the keyboard's LEDs state.
const KEYBOARD_LED: u8 = 0xed;
/// Command to set the keyboard's scancode set.
const KEYBOARD_SCANCODE: u8 = 0xf0;
/// Command to set the keyboard's typematic byte.
const KEYBOARD_TYPEMATIC: u8 = 0xf3;
/// Command to enable keyboard scanning.
const KEYBOARD_ENABLE: u8 = 0xf4;
/// Command to disable keyboard scaning.
const KEYBOARD_DISABLE: u8 = 0xf5;

/// Keyboard acknowledgement.
const KEYBOARD_ACK: u8 = 0xfa;
/// Response telling to resend the command.
const KEYBOARD_RESEND: u8 = 0xf4;

// TODO Turn commands and flags into constants.

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

	// Setting keyboard's scancode
	send_command(KEYBOARD_SCANCODE, KEYBOARD_ACK)?;
	keyboard_send(1)?;

	// Setting keyboard's typematic byte
	send_command(KEYBOARD_TYPEMATIC, KEYBOARD_ACK)?;
	keyboard_send(0)?;

	// Enabling keyboard scanning
	send_command(KEYBOARD_ENABLE, KEYBOARD_ACK)?;

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

/// Tests the first device.
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

/// Handles the given keyboard input.
/// `key` is the key that has been typed.
/// `action` is the action.
fn handle_input(key: KeyboardKey, action: KeyboardAction) {
	// TODO Do not retrieve at each keystroke
	if let Some(manager) = manager::get_by_name("kbd") {
		if let Some(manager) = manager.get_mut() {
			let mut guard = manager.lock(true);
			let manager = guard.get_mut();

			let kbd_manager = unsafe {
				&mut *(manager as *mut dyn DeviceManager as *mut KeyboardManager)
			};
			kbd_manager.input(key, action);

			if key == KeyboardKey::KeyPause && action == KeyboardAction::Pressed {
				kbd_manager.input(key, KeyboardAction::Released);
			}
		}
	}
}

/// The PS2 keyboard structure.
pub struct PS2Keyboard {
	/// The callback hook for keyboard input interrupts.
	keyboard_interrupt_callback_hook: Option<CallbackHook>,

	/// The state of LEDs.
	leds_state: u8,
}

impl PS2Keyboard {
	/// Creates the keyboard's instance.
	pub fn new() -> Result<Self, ()> {
		let mut s = Self {
			keyboard_interrupt_callback_hook: None,

			leds_state: 0,
		};
		s.init()?;
		Ok(s)
	}

	/// Initializes the handler.
	fn init(&mut self) -> Result<(), ()> {
		// TODO Check if PS/2 controller is existing using ACPI

		idt::wrap_disable_interrupts(|| {
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
					handle_input(key, action);
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
		})
	}
}

impl Keyboard for PS2Keyboard {
	fn set_led(&mut self, led: KeyboardLED, enabled: bool) {
		let offset = match led {
			KeyboardLED::NumberLock => 0,
			KeyboardLED::CapsLock => 1,
			KeyboardLED::ScrollLock => 2,
		};

		if enabled {
			self.leds_state |= 1 << offset;
		} else {
			self.leds_state &= !(1 << offset);
		}

		let _ = send_command(KEYBOARD_LED, KEYBOARD_ACK);
		let _ = keyboard_send(self.leds_state);
	}
}
