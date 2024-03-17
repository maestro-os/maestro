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

//! Time management implementation.
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

use crate::{event, event::CallbackResult};
use core::mem::ManuallyDrop;
use unit::{Timestamp, TimestampScale};
use utils::{boxed::Box, errno::EResult, lock::IntMutex, math::rational::Rational};

/// Atomic storage for a timestamp.
///
/// This wrapper is required because timestamps span 64 bits, but 32 bits architectures may not
/// support atomic operations on 64 bits operands.
pub struct AtomicTimestamp {
	#[cfg(not(target_has_atomic = "64"))]
	inner: IntMutex<Timestamp>,
	#[cfg(target_has_atomic = "64")]
	inner: AtomicU64,
}

impl AtomicTimestamp {
	pub const fn new(val: Timestamp) -> Self {
		Self {
			#[cfg(not(target_has_atomic = "64"))]
			inner: IntMutex::new(val),
			#[cfg(target_has_atomic = "64")]
			inner: AtomicU64::new(val),
		}
	}

	/// Loads and returns the value.
	#[inline]
	pub fn load(&self) -> Timestamp {
		#[cfg(not(target_has_atomic = "64"))]
		{
			*self.inner.lock()
		}
		#[cfg(target_has_atomic = "64")]
		{
			self.inner.load(core::sync::atomic::Ordering::Relaxed)
		}
	}

	/// Stores the given value and returns the previous.
	#[inline]
	pub fn store(&self, val: Timestamp) -> Timestamp {
		#[cfg(not(target_has_atomic = "64"))]
		{
			let mut guard = self.inner.lock();
			let prev = *guard;
			*guard = val;
			prev
		}
		#[cfg(target_has_atomic = "64")]
		{
			self.inner.store(val, core::sync::atomic::Ordering::Relaxed)
		}
	}

	/// Adds the given value and returns the previous.
	#[inline]
	pub fn fetch_add(&self, val: Timestamp) -> Timestamp {
		#[cfg(not(target_has_atomic = "64"))]
		{
			let mut guard = self.inner.lock();
			let prev = *guard;
			*guard = prev.wrapping_add(val);
			prev
		}
		#[cfg(target_has_atomic = "64")]
		{
			self.inner
				.fetch_add(val, core::sync::atomic::Ordering::Relaxed)
		}
	}
}

/// Initializes time management.
pub(crate) fn init() -> EResult<()> {
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
			// FIXME: the value is probably not right
			clock::update(i64::from(freq * 1_000_000_000) as _);
			timer::tick();

			CallbackResult::Continue
		})?;
		let _ = ManuallyDrop::new(hook);

		rtc.set_enabled(true);
	}

	Ok(())
}
