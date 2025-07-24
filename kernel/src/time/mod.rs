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
pub mod timer;
pub mod unit;

use crate::{
	process::{
		Process, State,
		scheduler::schedule,
		signal::{SIGEV_NONE, SigEvent},
	},
	time::{
		clock::{Clock, current_time_ns},
		timer::Timer,
		unit::TimeUnit,
	},
};
use core::hint::unlikely;
use unit::Timestamp;
use utils::{errno, errno::EResult};

/// Trait representing a hardware timer.
pub trait HwTimer {
	/// Enables or disable the timer.
	fn set_enabled(&mut self, enable: bool);
	/// Sets the timer's frequency in hertz.
	///
	/// The function selects the closest possible frequency to `freq`.
	fn set_frequency(&mut self, freq: u32);

	/// Returns the value of the timer, if applicable.
	fn get_value(&self) -> Option<Timestamp> {
		None
	}

	/// Returns the interrupt vector of the timer.
	fn get_interrupt_vector(&self) -> u32;
}

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
		schedule();
	}
	Ok(())
}

/// Initializes time management.
pub(crate) fn init() -> EResult<()> {
	// TODO initialize timekeeping
	Ok(())
}
