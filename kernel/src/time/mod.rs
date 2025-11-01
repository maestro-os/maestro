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
	arch::{
		core_id,
		x86::{apic, timer::rtc},
	},
	int,
	int::CallbackResult,
	process,
	process::{Process, State, scheduler::schedule},
	time::{
		clock::{Clock, current_time_ns},
		timer::Timer,
		unit::TimeUnit,
	},
};
use core::{hint::unlikely, mem::ManuallyDrop};
use unit::Timestamp;
use utils::{errno, errno::EResult};

/// Makes the current thread sleep for `delay`, in nanoseconds.
///
/// `clock` is the clock to use.
///
/// If the current process is interrupted by a signal, the function returns [`errno::EINTR`] and
/// sets the remaining time in `remain`.
pub fn sleep_for(clock: Clock, delay: Timestamp, remain: &mut Timestamp) -> EResult<()> {
	let proc = Process::current();
	// FIXME: there can be allocation failures here
	let mut timer = Timer::new(clock, move || {
		Process::wake_from(&proc, State::IntSleeping as u8)
	})?;
	timer.set_time(0, delay)?;
	// Loop until the timer expires
	loop {
		let cur_ts = current_time_ns(clock);
		if unlikely(timer.has_expired(cur_ts)) {
			break;
		}
		// The timer has not expired, we need to sleep
		if unlikely(Process::current().has_pending_signal()) {
			*remain = timer.get_time().it_value.to_nano();
			return Err(errno!(EINTR));
		}
		process::set_state(State::IntSleeping);
		schedule();
	}
	Ok(())
}

/// Initializes timekeeping
pub(crate) fn init() -> EResult<()> {
	clock::init(rtc::read_time());
	const FREQUENCY: u32 = 1024;
	rtc::set_frequency(FREQUENCY);
	if apic::is_present() {
		apic::redirect_int(0x8, core_id(), rtc::INTERRUPT_VECTOR);
	}
	let hook = int::register_callback(rtc::INTERRUPT_VECTOR as _, move |_, _, _, _| {
		rtc::reset();
		// FIXME: we are loosing precision here
		clock::update((1_000_000_000 / FREQUENCY) as _);
		timer::tick();
		CallbackResult::Continue
	})?;
	let _ = ManuallyDrop::new(hook);
	rtc::set_enabled(true);
	Ok(())
}
