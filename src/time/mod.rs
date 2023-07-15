//! This module handles time-releated features.
//!
//! The kernel stores a list of clock sources. A clock source is an object that
//! allow to get the current timestamp.

pub mod clock;
pub mod hw;
pub mod timer;
pub mod unit;

use crate::errno::EResult;
use crate::util::boxed::Box;
use unit::Timestamp;
use unit::TimestampScale;

/// Initializes time management.
pub fn init() -> EResult<()> {
	// Initialize hardware clocks
	let mut hw_clocks = hw::CLOCKS.lock();
	#[cfg(target_arch = "x86")]
	{
		hw_clocks.insert(b"pit".try_into()?, Box::new(hw::pit::PIT::new())?)?;
		hw_clocks.insert(b"rtc".try_into()?, Box::new(hw::rtc::RTC::new())?)?;
	}

	Ok(())
}
