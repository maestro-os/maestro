//! This module implements hardware clocks.

#[cfg(target_arch = "x86")]
pub mod pit;
#[cfg(target_arch = "x86")]
pub mod rtc;

use crate::time::unit::Timestamp;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::lock::Mutex;
use crate::util::math::rational::Rational;

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
