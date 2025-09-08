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

//! Timers implementation.

use super::unit::{ITimerspec32, TimerT};
use crate::{
	memory::oom,
	process::{
		Process, State,
		pid::Pid,
		signal::{SIGEV_NONE, SIGEV_SIGNAL, SIGEV_THREAD, SigEvent, Signal},
	},
	sync::mutex::IntMutex,
	time::{
		clock::{Clock, current_time_ns},
		unit::{TimeUnit, Timespec32, Timestamp},
	},
};
use core::hint::unlikely;
use utils::{
	boxed::Box,
	collections::{btreemap::BTreeMap, hashmap::HashMap, id_allocator::IDAllocator},
	errno,
	errno::{AllocResult, EResult},
	limits::TIMER_MAX,
};
// TODO make sure a timer doesn't send a signal to a thread that do not belong to the manager's
// process

#[derive(Default)]
struct TimerSpec {
	/// The timer's interval, in nanoseconds.
	interval: Timestamp,
	/// The next timestamp, in nanoseconds, at which the timer will expire.
	///
	/// If zero, the timer is unarmed.
	next: Option<Timestamp>,
}

struct TimerInner {
	/// The clock to user.
	clock: Clock,
	/// PID of the process to notify.
	pid: Pid,
	/// Definition of the action to perform when the timer is triggered.
	sevp: SigEvent,

	/// Timer setting.
	spec: IntMutex<TimerSpec>,
}

impl TimerInner {
	/// Tells whether the timer must be fired.
	///
	/// `cur_ts` is the current timestamp.
	#[inline]
	fn has_expired(&self, cur_ts: Timestamp) -> bool {
		self.spec
			.lock()
			.next
			.map(|next| cur_ts >= next)
			.unwrap_or(true)
	}

	/// Tells whether the timer is oneshot. If not, the timer repeats until manually stopped.
	#[inline]
	fn is_oneshot(&self) -> bool {
		self.spec.lock().interval == 0
	}

	/// Fires the timer.
	fn fire(&self) {
		let Some(proc) = Process::get_by_pid(self.pid) else {
			return;
		};
		match self.sevp.sigev_notify {
			SIGEV_NONE => Process::set_state(&proc, State::Running),
			SIGEV_SIGNAL => {
				let Ok(signal) = Signal::try_from(self.sevp.sigev_signo) else {
					return;
				};
				// TODO on sigint_t, set si_code to SI_TIMER
				proc.kill(signal);
			}
			SIGEV_THREAD => todo!(),
			_ => {}
		}
	}

	/// Resets the timer to be fired again.
	///
	/// Arguments:
	/// - `queue` is the queue.
	/// - `ts` is the current timestamp in nanoseconds.
	///
	/// On allocation error, the function returns an error.
	fn reset(
		&self,
		queue: &mut BTreeMap<(Timestamp, *const TimerInner), ()>,
		ts: Timestamp,
	) -> AllocResult<()> {
		let mut spec = self.spec.lock();
		// Remove from queue
		if let Some(next) = spec.next {
			queue.remove(&(next, self));
		}
		if spec.interval == 0 {
			spec.next = None;
		} else {
			let next = ts + spec.interval;
			spec.next = Some(next);
			// Insert back in queue
			queue.insert((next, self), ())?;
		}
		Ok(())
	}
}

/// A per-process timer.
pub struct Timer(Box<TimerInner>);

impl Timer {
	/// Creates a timer.
	///
	/// Arguments:
	/// - `clock` is the clock to use.
	/// - `pid` is the PID of the process to notify.
	/// - `sevp` describes the event to be triggered by the clock.
	pub fn new(clock: Clock, pid: Pid, sevp: SigEvent) -> EResult<Self> {
		// Validation
		if unlikely(!sevp.is_valid()) {
			return Err(errno!(EINVAL));
		}
		Ok(Self(Box::new(TimerInner {
			clock,
			pid,
			sevp,

			spec: Default::default(),
		})?))
	}

	/// Returns the current state of the timer.
	#[inline]
	pub fn get_time(&self) -> ITimerspec32 {
		let spec = self.0.spec.lock();
		let value = spec
			.next
			.map(|next| next.saturating_sub(current_time_ns(self.0.clock)))
			.unwrap_or(0);
		ITimerspec32 {
			it_interval: Timespec32::from_nano(spec.interval),
			it_value: Timespec32::from_nano(value),
		}
	}

	/// Sets the timer's state.
	///
	/// Arguments:
	/// - `interval` is the interval between two timer tick
	/// - `value` is the initial value of the timer
	///
	/// On allocation error, the function returns an error.
	pub fn set_time(&mut self, interval: Timestamp, value: Timestamp) -> AllocResult<()> {
		let mut queue = TIMERS_QUEUE.lock();
		let mut spec = self.0.spec.lock();
		// Remove from queue
		if let Some(next) = spec.next {
			queue.remove(&(next, self.0.as_ptr()));
		}
		// Update timer
		spec.interval = interval;
		// Arm or disarm
		if value == 0 {
			spec.next = None;
		} else {
			let next = current_time_ns(self.0.clock) + value;
			spec.next = Some(next);
			// Insert back in queue
			queue.insert((next, self.0.as_ptr()), ())?;
		}
		Ok(())
	}

	/// Tells whether the timer must be fired.
	///
	/// `cur_ts` is the current timestamp.
	#[inline]
	pub fn has_expired(&self, cur_ts: Timestamp) -> bool {
		self.0.has_expired(cur_ts)
	}
}

impl Drop for Timer {
	fn drop(&mut self) {
		let next = self.0.spec.lock().next;
		if let Some(next) = next {
			TIMERS_QUEUE.lock().remove(&(next, self.0.as_ptr()));
		}
	}
}

/// Manager for a process's timers.
pub struct TimerManager {
	/// The PID of the process to which the manager is associated.
	pid: Pid,

	/// ID allocator for timers.
	id_allocator: IDAllocator,
	/// The list of timers for the process. The key is the ID of the timer.
	timers: HashMap<u32, Timer>,
}

impl TimerManager {
	/// Creates a new instance.
	///
	/// `pid` is the PID of the process owning the timers.
	pub fn new(pid: Pid) -> AllocResult<Self> {
		Ok(Self {
			pid,

			id_allocator: IDAllocator::new(TIMER_MAX as _)?,
			timers: HashMap::new(),
		})
	}

	/// Creates a timer.
	///
	/// Arguments:
	/// - `clock` is the clock to use.
	/// - `sevp` describes the event to be triggered by the clock.
	///
	/// On success, the function returns the ID of the newly created timer.
	pub fn create_timer(&mut self, clock: Clock, sevp: SigEvent) -> EResult<u32> {
		let timer = Timer::new(clock, self.pid, sevp)?;
		let id = self.id_allocator.alloc(None)?;
		if let Err(e) = self.timers.insert(id, timer) {
			// Allocation error: rollback
			self.id_allocator.free(id);
			return Err(e.into());
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
	/// If the timer does not exist, the function returns an error.
	pub fn delete_timer(&mut self, id: TimerT) -> EResult<()> {
		self.timers
			.remove(&(id as _))
			.ok_or_else(|| errno!(EINVAL))?;
		Ok(())
	}
}

/// The queue of timers to be fired next.
///
/// The key has the following elements:
/// - the timestamp, in nanoseconds, at which the timer will fire next
/// - a pointer to the timer
static TIMERS_QUEUE: IntMutex<BTreeMap<(Timestamp, *const TimerInner), ()>> =
	IntMutex::new(BTreeMap::new());

/// Triggers all expired timers.
pub(super) fn tick() {
	let mut times: [Option<Timestamp>; 12] = Default::default();
	let mut queue = TIMERS_QUEUE.lock();
	loop {
		// Peek next timer
		let Some(((_, timer), _)) = queue.first_key_value() else {
			break;
		};
		let timer = unsafe { &**timer };
		// Get current time
		let ts = *times[timer.clock as usize].get_or_insert_with(|| current_time_ns(timer.clock));
		if !timer.has_expired(ts) {
			// If this timer has not expired, all the following timers won't be expired either
			break;
		}
		timer.fire();
		if timer.is_oneshot() {
			queue.pop_first();
		} else {
			oom::wrap(|| timer.reset(&mut queue, ts));
		}
	}
}
