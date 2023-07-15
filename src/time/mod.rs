//! This module implements time management.
//!
//! A clock is an object that gives the current time. A distinction has to be made between:
//! - Hardware Clocks, which are physical components (from the point of view of the kernel) which
//! give the ability to measure the passage of time, notably by producing interruptions at a given
//! frequency.
//! - Software Clocks, which maintain a timestamp based on hardware clocks.

pub mod clock;
pub mod hw;
pub mod timer;
pub mod unit;

use crate::errno::EResult;
use crate::event;
use crate::event::CallbackResult;
use crate::util::boxed::Box;
use crate::util::math::rational::Rational;
use core::mem::ManuallyDrop;
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
		// TODO implement HPET
		// TODO implement APIC timer
	}

	// Link hardware clock to software clock
	#[cfg(target_arch = "x86")]
	{
		let rtc = hw_clocks.get_mut(b"rtc".as_slice()).unwrap();
		let freq = Rational::from_frac(1, 1024);
		rtc.set_frequency(freq);

		let hook = event::register_callback(rtc.get_interrupt_vector(), move |_, _, _, _| {
			hw::rtc::RTC::reset();
			clock::update(i64::from(freq * 1000000000) as _);

			CallbackResult::Continue
		})?;
		let _ = ManuallyDrop::new(hook);

		rtc.set_enabled(true);
	}

	Ok(())
}
