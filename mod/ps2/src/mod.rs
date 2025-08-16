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

//! Personal System/2 (PS/2) is a connector designed for keyboards and mouses.
//! It has now been deprecated in favor of USB keyboards/mouses.

#![no_std]
#![no_main]

#[no_link]
extern crate kernel;

mod scancode;

use crate::scancode::ScancodeSet;
use core::any::Any;
use kernel::{
	arch::x86::{apic, apic::lapic_id, idt, idt::IntFrame, io},
	device::{
		keyboard::{Keyboard, KeyboardAction, KeyboardKey, KeyboardLED, KeyboardManager},
		manager,
	},
	int,
	int::{CallbackHook, CallbackResult},
	println,
	sync::mutex::Mutex,
};

kernel::module!([]);

/// The interrupt number for keyboard input events.
const KBD_INT: u8 = 0x21;

/// Register: Data
const DATA_REGISTER: u16 = 0x60;
/// Register: Status
const STATUS_REGISTER: u16 = 0x64;
/// Register: Command
const COMMAND_REGISTER: u16 = 0x64;

/// The maximum number of attempts for sending a command to the PS/2 controller.
const MAX_ATTEMPTS: usize = 3;

/// Command: Read configuration byte.
const CTRL_CMD_READ_CONFIG: u8 = 0x20;
/// Command: Write configuration byte.
const CTRL_CMD_WRITE_CONFIG: u8 = 0x60;
/// Comamnd: Disable second port.
const CTRL_CMD_DISABLE_PORT2: u8 = 0xa7;
/// Command: Test controller.
const CTRL_CMD_TEST_CONTROLLER: u8 = 0xaa;
/// Command: Test first port.
const CTRL_CMD_TEST_PORT1: u8 = 0xab;
/// Comamnd: Disable first port.
const CTRL_CMD_DISABLE_PORT1: u8 = 0xad;
/// Comamnd: Enable first port.
const CTRL_CMD_ENABLE_PORT1: u8 = 0xae;

/// Command: Set the keyboard's LEDs state.
const KBD_CMD_SET_LED: u8 = 0xed;
/// Command: Get or set the keyboard's scancode set.
const KBD_CMD_SCANCODE: u8 = 0xf0;
/// Command: Set the keyboard's typematic byte.
const KBD_CMD_SET_TYPEMATIC: u8 = 0xf3;
/// Command: Enable keyboard scanning.
const KBD_CMD_ENABLE: u8 = 0xf4;

/// Command response: Controller test passed.
const RESP_TEST_CONTROLLER_PASS: u8 = 0x55;
/// Command response: Keyboard test passed.
const RESP_TEST_KEYBOARD_PASS: u8 = 0x00;
/// Command response: Keyboard acknowledgement.
const RESP_KEYBOARD_ACK: u8 = 0xfa;

/// Tells whether the PS/2 registers are ready for reading.
fn can_read() -> bool {
	unsafe { io::inb(STATUS_REGISTER) & 0b1 != 0 }
}

/// Tells whether the PS/2 registers are ready for writing.
fn can_write() -> bool {
	unsafe { io::inb(STATUS_REGISTER) & 0b10 == 0 }
}

/// Waits until the registers are ready for reading.
fn wait_read() {
	while !can_read() {}
}

/// Waits until the registers are ready for reading.
fn wait_write() {
	while !can_write() {}
}

/// Waits for the data register to be ready, then reads from it.
fn read_data() -> u8 {
	wait_read();
	unsafe { io::inb(DATA_REGISTER) }
}

/// Clears the PS/2 controller's buffer.
fn clear_buffer() {
	while can_read() {
		unsafe {
			io::inb(DATA_REGISTER);
		}
	}
}

/// Waits for the data register to be ready, then writes to it.
fn write_data(n: u8) {
	wait_write();
	unsafe {
		io::outb(DATA_REGISTER, n);
	}
}

/// Waits for the command register to be ready, then writes the given command to it.
fn write_cmd(cmd: u8) {
	wait_write();
	unsafe {
		io::outb(COMMAND_REGISTER, cmd);
	}
}

/// Sends the given data `data` to the keyboard.
fn keyboard_send(data: u8) -> Result<(), ()> {
	for _ in 0..MAX_ATTEMPTS {
		write_data(data);
		let response = read_data();
		if response == RESP_KEYBOARD_ACK {
			return Ok(());
		}
	}
	Err(())
}

/// Sends the given command `command` to the controller.
///
/// The function returns successfully if the given `expected_response` is received.
fn send_command(command: u8, expected_response: u8) -> Result<(), ()> {
	for _ in 0..MAX_ATTEMPTS {
		write_cmd(command);
		let response = read_data();
		if response == expected_response {
			return Ok(());
		}
	}
	Err(())
}

/// Disables PS/2 devices.
fn disable_devices() {
	write_cmd(CTRL_CMD_DISABLE_PORT1);
	write_cmd(CTRL_CMD_DISABLE_PORT2);
}

/// Enables the keyboard device.
fn enable_keyboard(kbd: &mut PS2Keyboard) -> Result<(), ()> {
	write_cmd(CTRL_CMD_ENABLE_PORT1);

	// Set the keyboard's LEDs
	keyboard_send(KBD_CMD_SET_LED)?;
	keyboard_send(0)?;

	// FIXME
	// Get/set keyboard's scancode set
	/*ScancodeSet::current()?.fallback()?;
	let set = ScancodeSet::current()?;
	if !set.is_supported() {
		println!("Cannot use PS/2: scancode set not supported");
		return Err(());
	}*/
	let set = ScancodeSet::Set2;
	set.set_current()?;
	kbd.scancode_set = set;

	// Set keyboard's typematic byte
	keyboard_send(KBD_CMD_SET_TYPEMATIC)?;
	keyboard_send(0)?;

	// Enable keyboard scanning
	keyboard_send(KBD_CMD_ENABLE)?;

	Ok(())
}

/// Returns the configuration byte.
fn get_config_byte() -> u8 {
	write_cmd(CTRL_CMD_READ_CONFIG);
	read_data()
}

/// Sets the configuration byte.
fn set_config_byte(config: u8) {
	write_cmd(CTRL_CMD_WRITE_CONFIG);
	write_data(config);
}

/// Tests the PS/2 controller.
fn test_controller() -> Result<(), ()> {
	send_command(CTRL_CMD_TEST_CONTROLLER, RESP_TEST_CONTROLLER_PASS)
}

/// Tests the first device.
fn test_device() -> Result<(), ()> {
	send_command(CTRL_CMD_TEST_PORT1, RESP_TEST_KEYBOARD_PASS)
}

/// Handles the given keyboard input.
///
/// Arguments:
/// - `key` is the key that has been typed.
/// - `action` is the action.
fn handle_input(key: KeyboardKey, action: KeyboardAction) {
	// TODO Do not retrieve at each keystroke
	let Some(manager_mutex) = manager::get::<KeyboardManager>() else {
		return;
	};
	let mut manager = manager_mutex.lock();
	let kbd_manager = (&mut *manager as &mut dyn Any)
		.downcast_mut::<KeyboardManager>()
		.unwrap();
	kbd_manager.input(key, action);
}

/// Global variable containing the module's instance.
static PS2_KEYBOAD: Mutex<PS2Keyboard> = Mutex::new(PS2Keyboard {
	keyboard_interrupt_callback_hook: None,

	scancode_set: ScancodeSet::Set2,
	leds_state: 0,
});

/// The PS2 keyboard structure.
pub struct PS2Keyboard {
	/// The callback hook for keyboard input interrupts.
	keyboard_interrupt_callback_hook: Option<CallbackHook>,

	/// The current scancode set being used by the keyboard.
	scancode_set: ScancodeSet,
	/// The state of LEDs.
	leds_state: u8,
}

impl Keyboard for PS2Keyboard {
	fn set_led(&mut self, led: KeyboardLED, enabled: bool) {
		let offset = match led {
			KeyboardLED::ScrollLock => 0,
			KeyboardLED::NumberLock => 1,
			KeyboardLED::CapsLock => 2,
		};

		if enabled {
			self.leds_state |= 1 << offset;
		} else {
			self.leds_state &= !(1 << offset);
		}

		let _ = keyboard_send(KBD_CMD_SET_LED);
		let _ = keyboard_send(self.leds_state);
	}
}

fn init_in() -> Result<(), ()> {
	// TODO Check if PS/2 controller exists using ACPI

	let mut kbd = PS2_KEYBOAD.lock();

	idt::wrap_disable_interrupts(|| {
		disable_devices();
		clear_buffer();

		// Disable first and second port
		set_config_byte(get_config_byte() & 0b110100);

		println!("Test PS/2 controller...");
		test_controller()?;
		println!("Test PS/2 keyboard...");
		test_device()?;
		println!("Enable PS/2 keyboard...");
		enable_keyboard(&mut kbd)?;

		// Enable first port and disable keycodes translation
		set_config_byte((get_config_byte() | 0b1) & !(1 << 6));

		clear_buffer();
		Ok(())
	})?;

	let callback = |_id: u32, _code: u32, _regs: &mut IntFrame, _ring: u8| {
		let kbd = PS2_KEYBOAD.lock();
		while can_read() {
			if let Some((key, action)) = kbd.scancode_set.read_keystroke() {
				handle_input(key, action);
			}
		}
		CallbackResult::Continue
	};

	if apic::is_present() {
		apic::redirect_int(0x1, lapic_id(), KBD_INT);
	}
	let hook_result = int::register_callback(KBD_INT as _, callback);
	kbd.keyboard_interrupt_callback_hook = hook_result.map_err(|_| ())?;

	Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn init() -> bool {
	match init_in() {
		Ok(_) => {
			println!("PS/2 keyboard ready");
			true
		}
		Err(_) => {
			println!("Failed to initialize PS2 keyboard!");
			false
		}
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn fini() {
	// Destroy interrupt handler
	PS2_KEYBOAD.lock().keyboard_interrupt_callback_hook = None;
}
