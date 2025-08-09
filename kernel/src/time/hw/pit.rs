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

//! This module handles the PIT (Programmable Interrupt Timer) which allows to
//! trigger interruptions at a fixed interval.

use super::HwClock;
use crate::{
	arch,
	arch::x86::{idt, io::outb},
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

// FIXME prevent having several instances at the same time

/// The PIT.
pub struct PIT {}

impl PIT {
	/// Creates a new instance.
	///
	/// By default, the timer is disabled and its frequency is undefined.
	#[allow(clippy::new_without_default)]
	pub fn new() -> Self {
		let mut s = Self {};
		s.set_enabled(false);
		idt::wrap_disable_interrupts(|| unsafe {
			outb(
				PIT_COMMAND,
				SELECT_CHANNEL_0 | ACCESS_LOBYTE_HIBYTE | MODE_3,
			);
			s.set_frequency(1);
		});
		s
	}
}

impl HwClock for PIT {
	fn set_enabled(&mut self, enable: bool) {
		if enable {
			arch::enable_irq(0x0);
		} else {
			arch::disable_irq(0x0);
		}
	}

	fn set_frequency(&mut self, frequency: u32) {
		let mut count = if frequency != 0 {
			(BASE_FREQUENCY / frequency) as u16
		} else {
			0
		};
		if count == 0xffff {
			count = 0;
		}

		// Update frequency divider's value
		idt::wrap_disable_interrupts(|| unsafe {
			outb(CHANNEL_0, (count & 0xff) as u8);
			outb(CHANNEL_0, ((count >> 8) & 0xff) as u8);
		});
	}

	fn get_interrupt_vector(&self) -> u32 {
		0x20
	}
}

impl Drop for PIT {
	fn drop(&mut self) {
		self.set_enabled(false);
	}
}
