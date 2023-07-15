//! This module implements timers.

use crate::errno::Errno;
use crate::limits;
use crate::process::signal::SigEvent;
use crate::time::unit::ClockIdT;
use crate::time::unit::ITimerspec32;
use crate::time::unit::TimerT;
use crate::util::container::hashmap::HashMap;
use crate::util::container::id_allocator::IDAllocator;

// TODO make sure a timer doesn't send a signal to a thread that do not belong to the manager's
// process

/// Structure representing a per-process timer.
pub struct Timer {
	/// The ID of the clock to use.
	clockid: ClockIdT,
	/// Definition of the action to perform when the timer is triggered.
	sevp: SigEvent,

	/// The current state of the timer.
	time: ITimerspec32,
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

			time: ITimerspec32::default(),
		}
	}

	/// Tells whether the timer is armed.
	pub fn is_armed(&self) -> bool {
		self.time.it_value.tv_sec != 0 || self.time.it_value.tv_nsec != 0
	}

	/// Returns the current state of the timer.
	pub fn get_time(&self) -> ITimerspec32 {
		self.time.clone()
	}

	/// Sets the timer's state.
	pub fn set_time(&mut self, time: ITimerspec32) {
		self.time = time;
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

	/// Returns a mutable reference to the timer with the given ID.
	///
	/// If the timer doesn't exist, the function returns `None`.
	pub fn get_timer_mut(&mut self, id: TimerT) -> Option<&mut Timer> {
		self.timers.get_mut(&(id as _))
	}

	/// Deletes the timer with the given ID.
	///
	/// If the timer doesn't exist, the function returns an error.
	pub fn delete_timer(&mut self, id: TimerT) -> Result<(), Errno> {
		self.timers
			.remove(&(id as _))
			.ok_or_else(|| errno!(EINVAL))?;
		Ok(())
	}
}

/// Ticks active timers and triggers them if necessary.
pub(super) fn tick() {
	// TODO
}
