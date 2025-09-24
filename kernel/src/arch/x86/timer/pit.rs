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

//! PIT (Programmable Interrupt Timer) implementation.

use crate::arch::{
	disable_irq, enable_irq,
	x86::{idt::disable_int, io::outb},
};

/// PIT channel number 0.
const CHANNEL_0: u16 = 0x40;
/// PIT channel number 2.
const CHANNEL_2: u16 = 0x42;
/// The port to send a command to the PIT.
const PIT_COMMAND: u16 = 0x43;

/// The command to enable the PC speaker.
const BEEPER_ENABLE_COMMAND: u8 = 0x61;

/// Select PIT channel 0.
const SELECT_CHANNEL_0: u8 = 0b00 << 6;
/// Select PIT channel 2.
const SELECT_CHANNEL_2: u8 = 0b10 << 6;

/// Tells the PIT to read the whole counter value.
const ACCESS_LOBYTE_HIBYTE: u8 = 0b11 << 4;

/// Square wave generator.
const MODE_3: u8 = 0b011 << 1;

/// The base frequency of the PIT.
const BASE_FREQUENCY: u32 = 1193182;

/// Interrupt vector for the PIT.
pub const INTERRUPT_VECTOR: u8 = 0x20;

/// Initializes the PIT.
pub fn init(freq: u32) {
	disable_int(|| unsafe {
		outb(
			PIT_COMMAND,
			SELECT_CHANNEL_0 | ACCESS_LOBYTE_HIBYTE | MODE_3,
		);
		set_frequency(freq);
	});
}

/// Enables or disables the PIT.
pub fn set_enabled(enable: bool) {
	if enable {
		enable_irq(0x20);
	} else {
		disable_irq(0x20);
	}
}

/// Sets the PIT's frequency.
pub fn set_frequency(freq: u32) {
	let mut count = if freq != 0 {
		(BASE_FREQUENCY / freq) as u16
	} else {
		0
	};
	if count == 0xffff {
		count = 0;
	}
	// Update frequency divider's value
	disable_int(|| unsafe {
		outb(CHANNEL_0, (count & 0xff) as u8);
		outb(CHANNEL_0, ((count >> 8) & 0xff) as u8);
	});
}
