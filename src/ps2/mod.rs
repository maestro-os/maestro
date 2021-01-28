/// This module handles PS/2 devices.
/// TODO doc

use crate::io;

/// TODO doc
const DATA_REGISTER: u16 = 0x60;
/// TODO doc
const STATUS_REGISTER: u16 = 0x64;
/// TODO doc
const COMMAND_REGISTER: u16 = 0x64;

/// Enumeration of keyboard actions.
pub enum KeyboardAction {
	/// The key was pressed.
	Pressed,
	/// The key was released.
	Released,
	/// The key was repeated.
	Repeated,
}

/// Tells whether the PS/2 buffer is ready for reading.
fn can_read() -> bool {
	unsafe { // IO operation
		io::inb(STATUS_REGISTER) & 0b1 != 0
	}
}

/// Tells whether the PS/2 buffer is ready for writing.
fn can_write() -> bool {
	unsafe { // IO operation
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
		unsafe { // IO operation
			io::inb(DATA_REGISTER);
		}
	}
}

/// Enables PS/2 devices.
fn enable_devices() {
	// TODO
}

/// Disables PS/2 devices.
fn disable_devices() {
	// TODO
}

/// Initializes the PS/2 driver.
pub fn init() {
	// TODO Check if existing using ACPI
	disable_devices();
	clear_buffer();
	// TODO
}

/// Sets the callback for keyboard actions.
pub fn set_keyboard_callback<F: FnMut(char, KeyboardAction)>(_f: F) {
	// TODO
}
