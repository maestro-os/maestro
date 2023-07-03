//! This module implements timers.

use crate::process::signal::SigEvent;
use core::ffi::c_int;
use core::ffi::c_void;

pub mod pit;

/// Equivalent of POSIX `clockid_t`.
pub type ClockIdT = c_int;
/// Equivalent of POSIX `timer_t`.
pub type TimerT = *mut c_void;

// TODO make sure a timer doesn't send a signal to a thread that do not belong to the manager's
// process

/// Structure managing a process's timers.
#[derive(Default)]
pub struct TimerManager {
	// TODO
}

impl TimerManager {
	/// Creates a timer.
	///
	/// Arguments:
	/// - `clockid` is the ID of the clock to use.
	/// - `sevp` describes the event to be triggered by the clock.
	pub fn create_timer(&mut self, _clockid: ClockIdT, _sevp: SigEvent) -> u32 {
		// TODO
		todo!()
	}
}
