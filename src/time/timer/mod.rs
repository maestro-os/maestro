//! This module implements timers.

use crate::errno::Errno;
use crate::limits;
use crate::process::signal::SigEvent;
use crate::util::container::hashmap::HashMap;
use crate::util::container::id_allocator::IDAllocator;
use core::ffi::c_int;
use core::ffi::c_void;

pub mod pit;

/// Equivalent of POSIX `clockid_t`.
pub type ClockIdT = c_int;
/// Equivalent of POSIX `timer_t`.
pub type TimerT = *mut c_void;

// TODO make sure a timer doesn't send a signal to a thread that do not belong to the manager's
// process

/// Structure representing a per-process timer.
pub struct Timer {
	/// The ID of the clock to use.
	clockid: ClockIdT,
	/// Definition of the action to perform when the timer is triggered.
	sevp: SigEvent,
}

impl Timer {
	/// Creates a timer.
	///
	/// Arguments:
	/// - `clockid` is the ID of the clock to use.
	/// - `sevp` describes the event to be triggered by the clock.
	pub fn new(clockid: ClockIdT, sevp: SigEvent) -> Self {
		// TODO check clock is valid

		Self {
			clockid,
			sevp,
		}
	}
}

/// Structure managing a process's timers.
pub struct TimerManager {
	/// ID allocator for timers.
	id_allocator: IDAllocator,
	/// The list of timers for the process. The key is the ID of the timer.
	timers: HashMap<u32, Timer>,
}

impl TimerManager {
	/// Creates a manager.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			id_allocator: IDAllocator::new(limits::TIMER_MAX as _)?,
			timers: HashMap::new(),
		})
	}

	/// Creates a timer.
	///
	/// Arguments:
	/// - `clockid` is the ID of the clock to use.
	/// - `sevp` describes the event to be triggered by the clock.
	///
	/// On success, the function returns the ID of the newly created timer.
	pub fn create_timer(&mut self, clockid: ClockIdT, sevp: SigEvent) -> Result<u32, Errno> {
		let timer = Timer::new(clockid, sevp);
		let id = self.id_allocator.alloc(None)?;
		if let Err(e) = self.timers.insert(id, timer) {
			self.id_allocator.free(id);
			return Err(e);
		}

		Ok(id)
	}
}
