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
use crate::{idt, idt::pic, io};
use utils::math::rational::Rational;

/// PIT channel number 0.
const CHANNEL_0: u16 = 0x40;
/// PIT channel number 1.
const CHANNEL_1: u16 = 0x41;
/// PIT channel number 2.
const CHANNEL_2: u16 = 0x42;
/// The port to send a command to the PIT.
const PIT_COMMAND: u16 = 0x43;

/// The command to enable the PC speaker.
const BEEPER_ENABLE_COMMAND: u8 = 0x61;

/// Select PIT channel 0.
const SELECT_CHANNEL_0: u8 = 0b00 << 6;
/// Select PIT channel 1.
const SELECT_CHANNEL_1: u8 = 0b01 << 6;
/// Select PIT channel 2.
const SELECT_CHANNEL_2: u8 = 0b10 << 6;
/// The read back command, used to read the current state of the PIT (doesn't
/// work on 8253 and older).
const READ_BACK_COMMAND: u8 = 0b11 << 6;

/// Tells the PIT to copy the current count to the latch register to be read by
/// the CPU.
const ACCESS_LATCH_COUNT_VALUE: u8 = 0b00 << 4;
/// Tells the PIT to read only the lowest 8 bits of the counter value.
const ACCESS_LOBYTE: u8 = 0b01 << 4;
/// Tells the PIT to read only the highest 8 bits of the counter value.
const ACCESS_HIBYTE: u8 = 0b10 << 4;
/// Tells the PIT to read the whole counter value.
const ACCESS_LOBYTE_HIBYTE: u8 = 0b11 << 4;

/// Interrupt on terminal count.
const MODE_0: u8 = 0b000 << 1;
/// Hardware re-triggerable one-shot.
const MODE_1: u8 = 0b001 << 1;
/// Rate generator.
const MODE_2: u8 = 0b010 << 1;
/// Square wave generator.
const MODE_3: u8 = 0b011 << 1;
/// Software triggered strobe.
const MODE_4: u8 = 0b100 << 1;
/// Hardware triggered strobe.
const MODE_5: u8 = 0b101 << 1;

/// Tells whether the BCD mode is enabled.
const BCD_MODE: u8 = 0b1;

/// The base frequency of the PIT.
const BASE_FREQUENCY: Rational = Rational::from_integer(1193182);

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
			io::outb(
				PIT_COMMAND,
				SELECT_CHANNEL_0 | ACCESS_LOBYTE_HIBYTE | MODE_3,
			);

			s.set_frequency(Rational::from(1));
		});

		s
	}
}

impl HwClock for PIT {
	fn set_enabled(&mut self, enable: bool) {
		if enable {
			pic::enable_irq(0x0);
		} else {
			pic::disable_irq(0x0);
		}
	}

	fn set_frequency(&mut self, frequency: Rational) {
		let mut count = if frequency != Rational::from(0) {
			i64::from(BASE_FREQUENCY / frequency) as u16
		} else {
			0
		};
		if count == 0xffff {
			count = 0;
		}

		// Update frequency divider's value
		idt::wrap_disable_interrupts(|| unsafe {
			io::outb(CHANNEL_0, (count & 0xff) as u8);
			io::outb(CHANNEL_0, ((count >> 8) & 0xff) as u8);
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
