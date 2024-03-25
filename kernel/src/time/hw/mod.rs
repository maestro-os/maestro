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

//! This module implements hardware clocks.

#[cfg(target_arch = "x86")]
pub mod pit;
#[cfg(target_arch = "x86")]
pub mod rtc;

use crate::time::unit::Timestamp;
use utils::{
	boxed::Box,
	collections::{hashmap::HashMap, string::String},
	lock::Mutex,
	math::rational::Rational,
};

/// Trait representing a hardware clock.
pub trait HwClock {
	/// Enables or disable the clock.
	fn set_enabled(&mut self, enable: bool);
	/// Sets the clock's frequency.
	///
	/// The actual frequency is the closest possible rounded down according to the clock's
	/// resolution.
	fn set_frequency(&mut self, freq: Rational);

	/// Returns the value of the clock, if applicable.
	fn get_value(&self) -> Option<Timestamp> {
		None
	}

	/// Returns the interrupt vector of the timer.
	fn get_interrupt_vector(&self) -> u32;
}

/// The list of hardware clock sources.
///
/// The key is the name of the clock.
pub static CLOCKS: Mutex<HashMap<String, Box<dyn HwClock>>> = Mutex::new(HashMap::new());
