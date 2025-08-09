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

//! RTC (Real Time Clock) implementation.

use crate::{
	arch::x86::{
		idt,
		io::{inb, outb},
	},
	time::HwTimer,
};

/// The ID of the port used to select the CMOS register to read.
const SELECT_PORT: u16 = 0x70;
/// The ID of the port to read or write a CMOS port previously selected.
const VALUE_PORT: u16 = 0x71;

/// The ID of the status register A.
const STATUS_A_REGISTER: u8 = 0x0a;
/// The ID of the status register B.
const STATUS_B_REGISTER: u8 = 0x0b;
/// The ID of the status register C.
const STATUS_C_REGISTER: u8 = 0x0c;

/// Structure representing the RTC
///
/// **Note**: the RTC needs a call to `reset` to allow the next tick to be fired.
pub struct Rtc;

impl Rtc {
	/// Resets the timer to make it ready for the next tick.
	#[inline]
	pub fn reset() {
		unsafe {
			outb(SELECT_PORT, STATUS_C_REGISTER);
			inb(VALUE_PORT);
		}
	}
}

impl HwTimer for Rtc {
	fn set_enabled(&mut self, enable: bool) {
		idt::wrap_disable_interrupts(|| unsafe {
			outb(SELECT_PORT, STATUS_B_REGISTER | 0x80);
			let prev = inb(VALUE_PORT);
			outb(SELECT_PORT, STATUS_B_REGISTER | 0x80);
			if enable {
				outb(VALUE_PORT, prev | 0x40);
			} else {
				outb(VALUE_PORT, prev & !0x40);
			}
		});
	}

	fn set_frequency(&mut self, freq: u32) {
		let rate = (32768u32 / freq).trailing_zeros() as u8 + 1;
		idt::wrap_disable_interrupts(|| unsafe {
			outb(SELECT_PORT, STATUS_A_REGISTER | 0x80);
			let prev = inb(VALUE_PORT);
			outb(SELECT_PORT, STATUS_A_REGISTER | 0x80);
			outb(VALUE_PORT, (prev & 0xf0) | (rate & 0x0f));
		});
	}

	fn get_interrupt_vector(&self) -> u32 {
		0x28
	}
}

impl Drop for Rtc {
	fn drop(&mut self) {
		self.set_enabled(false);
	}
}

/// Initializes the RTC.
pub fn init() -> Rtc {
	let mut rtc = Rtc;
	rtc.set_enabled(false);
	rtc
}
