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
//!   give the ability to measure the passage of time, notably by producing interruptions at a
//!   given frequency.
//! - Software Clocks, which maintain a timestamp based on hardware clocks.

pub mod clock;
pub mod hw;
pub mod timer;
pub mod unit;

use crate::{
	event,
	event::CallbackResult,
	process::{
		Process, State,
		scheduler::Scheduler,
		signal::{SIGEV_NONE, SigEvent},
	},
	time::{
		clock::{Clock, current_time_ns},
		timer::Timer,
		unit::TimeUnit,
	},
};
use core::{hint::unlikely, mem::ManuallyDrop};
use unit::Timestamp;
use utils::{boxed::Box, errno, errno::EResult};

/// Timer frequency.
const FREQUENCY: u32 = 1024;

/// Makes the current thread sleep for `delay`, in nanoseconds.
///
/// `clock` is the clock to use.
///
/// If the current process is interrupted by a signal, the function returns [`errno::EINTR`] and
/// sets the remaining time in `remain`.
pub fn sleep_for(clock: Clock, delay: Timestamp, remain: &mut Timestamp) -> EResult<()> {
	// Setup timer
	let pid = Process::current().get_pid();
	// FIXME: there can be allocation failures here
	let mut timer = Timer::new(
		clock,
		pid,
		SigEvent {
			sigev_notify: SIGEV_NONE,
			..Default::default()
		},
	)?;
	timer.set_time(0, delay)?;
	// Loop until the timer expires
	loop {
		let cur_ts = current_time_ns(clock);
		if unlikely(timer.has_expired(cur_ts)) {
			break;
		}
		// The timer has not expired, we need to sleep
		{
			let proc = Process::current();
			if proc.has_pending_signal() {
				*remain = timer.get_time().it_value.to_nano();
				return Err(errno!(EINTR));
			}
			proc.set_state(State::Sleeping);
		}
		Scheduler::tick();
	}
	Ok(())
}

/// Initializes time management.
pub(crate) fn init() -> EResult<()> {
	// Initialize hardware clocks
	let mut hw_clocks = hw::CLOCKS.lock();
	hw_clocks.insert(b"pit".try_into()?, Box::new(hw::pit::PIT::new())?)?;
	hw_clocks.insert(b"rtc".try_into()?, Box::new(hw::rtc::RTC::new())?)?;
	// TODO implement HPET
	// TODO implement APIC timer
	// Link hardware clock to software clock
	let rtc = hw_clocks.get_mut(b"rtc".as_slice()).unwrap();
	rtc.set_frequency(FREQUENCY);
	let hook = event::register_callback(rtc.get_interrupt_vector(), move |_, _, _, _| {
		hw::rtc::RTC::reset();
		// FIXME: we are loosing precision here
		clock::update((1_000_000_000 / FREQUENCY) as _);
		timer::tick();
		CallbackResult::Continue
	})?;
	let _ = ManuallyDrop::new(hook);
	rtc.set_enabled(true);
	Ok(())
}
