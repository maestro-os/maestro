//! This module implements timers.

use crate::errno::Errno;
use crate::limits;
use crate::process::pid::Pid;
use crate::process::signal::SigEvent;
use crate::process::Process;
use crate::time::unit::ClockIdT;
use crate::time::unit::ITimerspec32;
use crate::time::unit::TimeUnit;
use crate::time::unit::TimerT;
use crate::time::unit::Timespec;
use crate::util::container::hashmap::HashMap;
use crate::util::container::id_allocator::IDAllocator;
use crate::util::container::map::Map;
use crate::util::lock::IntMutex;

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
	#[inline]
	pub fn is_armed(&self) -> bool {
		!self.time.it_value.is_zero()
	}

	/// Tells whether the timer must be fired.
	#[inline]
	pub fn is_expired(&self) -> bool {
		// TODO
		todo!()
	}

	/// Tells whether the timer is oneshot. If not, the timer repeats until manually stopped.
	#[inline]
	pub fn is_oneshot(&self) -> bool {
		self.time.it_interval.is_zero()
	}

	/// Returns the current state of the timer.
	#[inline]
	pub fn get_time(&self) -> ITimerspec32 {
		self.time.clone()
	}

	/// Sets the timer's state.
	#[inline]
	pub fn set_time(&mut self, time: ITimerspec32) {
		// TODO update queue (lookup using previous value)

		self.time = time;
	}

	/// Fires the timer.
	pub fn fire(&mut self) {
		// TODO
		todo!()
	}
}

/// Structure managing a process's timers.
pub struct TimerManager {
	/// The PID of the process to which the manager is associated.
	pid: Pid,

	/// ID allocator for timers.
	id_allocator: IDAllocator,
	/// The list of timers for the process. The key is the ID of the timer.
	timers: HashMap<u32, Timer>,
}

impl TimerManager {
	/// Creates a manager.
	pub fn new(pid: Pid) -> Result<Self, Errno> {
		Ok(Self {
			pid,

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

impl Drop for TimerManager {
	fn drop(&mut self) {
		let mut queue = TIMERS_QUEUE.lock();
		queue.retain(|_, (pid, _)| *pid != self.pid);
	}
}

// FIXME: not adapted since the current contain only allows one timer per possible value
/// The queue of timers to be fired next.
///
/// - key: the timestamp at which the timer will fire next
/// - value: a tuple with the PID of the process owning the timer and the ID of the timer
static TIMERS_QUEUE: IntMutex<Map<Timespec, (Pid, TimerT)>> = IntMutex::new(Map::new());

/// Ticks active timers and triggers them if necessary.
pub(super) fn tick() {
	let mut queue = TIMERS_QUEUE.lock();

	loop {
		// Peek next timer
		let Some((_, (pid, timer_id))) = queue.first_key_value() else {
            break;
        };

		// Get process
		let Some(proc_mutex) = Process::get_by_pid(*pid) else {
            // invalid timer, remove
			queue.pop_first();
            break;
        };

		// Get timer manager
		let timer_manager_mutex = {
			let proc = proc_mutex.lock();
			proc.timer_manager()
		};
		let mut timer_manager = timer_manager_mutex.lock();

		// Get timer
		let Some(timer) = timer_manager.get_timer_mut(*timer_id) else {
            // invalid timer, remove
			queue.pop_first();
            break;
        };

		if !timer.is_expired() {
			// If this timer has not expired, all the next timers won't be expired either
			break;
		}

		timer.fire();

		if timer.is_oneshot() {
			queue.pop_first();
		} else {
			// TODO update key
			todo!()
		}
	}
}
