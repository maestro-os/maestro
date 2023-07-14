//! This module handles time-releated features.
//!
//! The kernel stores a list of clock sources. A clock source is an object that
//! allow to get the current timestamp.

pub mod clock;
pub mod hw;
pub mod timer;
pub mod unit;

use crate::errno::EResult;
use crate::time::clock::CLOCK_MONOTONIC;
use crate::time::clock::CLOCK_REALTIME;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use unit::Timestamp;
use unit::TimestampScale;

/// Initializes time management.
pub fn init() -> EResult<()> {
	// Initialize hardware clocks
	let mut hw_clocks = hw::CLOCKS.lock();
	#[cfg(target_arch = "x86")]
	hw_clocks.insert(String::try_from(b"pit")?, Box::new(hw::pit::PIT::new())?)?;
	// TODO register all

	// Initializes software clocks
	let mut clocks = clock::CLOCKS.lock();
	clocks.insert(
		CLOCK_REALTIME,
		Box::new(clock::realtime::ClockRealtime::new(false))?,
	)?;
	clocks.insert(
		CLOCK_MONOTONIC,
		Box::new(clock::realtime::ClockRealtime::new(true))?,
	)?;
	// TODO register all

	Ok(())
}
