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

use super::unit::TimerT;
use crate::{
	memory::oom,
	process::{
		Process,
		signal::{SIGEV_SIGNAL, SIGEV_THREAD, SIGEV_THREAD_ID, SigEvent, Signal},
	},
	sync::spin::IntSpin,
	time::{
		clock::{Clock, current_time_ns},
		unit::Timestamp,
	},
};
use core::{ffi::c_int, hint::unlikely};
use utils::{
	boxed::Box,
	collections::{btreemap::BTreeMap, hashmap::HashMap, id_allocator::IDAllocator},
	errno,
	errno::{AllocResult, EResult},
	limits::TIMER_MAX,
};

/// `setitimer`/`getitimer`: real-time interval timer, decrementing in real time and firing
/// `SIGALRM`.
pub const ITIMER_REAL: c_int = 0;
/// `setitimer`/`getitimer`: virtual interval timer, decrementing while the process executes in
/// userspace and firing `SIGVTALRM`.
pub const ITIMER_VIRTUAL: c_int = 1;
/// `setitimer`/`getitimer`: profiling interval timer, decrementing while the process executes and
/// firing `SIGPROF`.
pub const ITIMER_PROF: c_int = 2;

// TODO make sure a timer doesn't send a signal to a thread that do not belong to the manager's
// process

#[derive(Default)]
struct TimerSpec {
	/// The timer's interval, in nanoseconds
	interval: Timestamp,
	/// The next timestamp, in nanoseconds, at which the timer will expire
	///
	/// If zero, the timer is unarmed
	next: Option<Timestamp>,
}

struct TimerInner {
	/// The clock to use
	clock: Clock,
	/// Timer setting
	spec: IntSpin<TimerSpec>,
	/// The function to execute when the timer expires
	f: Box<dyn Fn()>,
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
	/// - `clock` is the clock to use
	/// - `f` is the function to execute when the timer fires
	pub fn new<F: 'static + Fn()>(clock: Clock, f: F) -> AllocResult<Self> {
		Ok(Self(Box::new(TimerInner {
			clock,
			spec: Default::default(),
			f: Box::new(f)?,
		})?))
	}

	/// Returns the current state of the timer as `(interval_ns, value_ns)` where `value_ns`
	/// is the nanoseconds remaining until the next firing (`0` if disarmed).
	#[inline]
	pub fn get_time(&self) -> (Timestamp, Timestamp) {
		let spec = self.0.spec.lock();
		let value = spec
			.next
			.map(|next| next.saturating_sub(current_time_ns(self.0.clock)))
			.unwrap_or(0);
		(spec.interval, value)
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
	/// ID allocator for timers
	id_allocator: IDAllocator,
	/// The list of timers for the process. The key is the ID of the timer
	timers: HashMap<TimerT, Timer>,
	/// The process's interval timers, indexed by `which` (`ITIMER_REAL`, `ITIMER_VIRTUAL`,
	/// `ITIMER_PROF`)
	itimers: [Option<Timer>; 3],
}

impl TimerManager {
	/// Creates a new instance.
	pub(crate) fn new() -> AllocResult<Self> {
		Ok(Self {
			id_allocator: IDAllocator::new_allocated(TIMER_MAX as _)?,
			timers: HashMap::new(),
			itimers: Default::default(),
		})
	}

	/// Returns a mutable reference to the timer with the given ID.
	///
	/// If the timer doesn't exist, the function returns `None`.
	#[inline]
	pub fn get_timer_mut(&mut self, id: TimerT) -> Option<&mut Timer> {
		self.timers.get_mut(&id)
	}

	/// Creates a timer for the current process.
	///
	/// Arguments:
	/// - `clock` is the clock to use
	/// - `sevp` describes the event to be triggered by the clock
	///
	/// On success, the function returns the ID of the newly created timer.
	pub fn create_timer(clock: Clock, sevp: SigEvent) -> EResult<u32> {
		if unlikely(!sevp.is_valid()) {
			return Err(errno!(EINVAL));
		}
		let sig = Signal::try_from(sevp.sigev_signo)?;
		let proc = Process::current();
		let f = move || {
			match sevp.sigev_notify {
				// TODO for SIGEV_THREAD_ID, target the thread identified by
				// sevp.sigev_notify_thread_id
				SIGEV_SIGNAL | SIGEV_THREAD_ID => {
					// TODO on sigint_t, set si_code to SI_TIMER
					Process::kill(&proc, sig);
				}
				SIGEV_THREAD => todo!(),
				_ => {}
			}
		};
		let timer = Timer::new(clock, f)?;
		let proc = Process::current();
		let mut this = proc.timer_manager.lock();
		let id = this.id_allocator.alloc(None)?;
		if let Err(e) = this.timers.insert(id as _, timer) {
			// Allocation error: rollback
			this.id_allocator.free(id);
			return Err(e.into());
		}
		Ok(id)
	}

	/// Deletes the timer with the given ID.
	///
	/// If the timer does not exist, the function returns an error.
	pub fn delete_timer(id: TimerT) -> EResult<()> {
		Process::current()
			.timer_manager
			.lock()
			.timers
			.remove(&id)
			.ok_or_else(|| errno!(EINVAL))?;
		Ok(())
	}

	/// Returns the clock and signal associated with the interval timer `which`.
	///
	/// If `which` is not a valid interval timer, the function returns `EINVAL`.
	fn itimer_clock_signal(which: c_int) -> EResult<(Clock, Signal)> {
		match which {
			ITIMER_REAL => Ok((Clock::Monotonic, Signal::SIGALRM)),
			ITIMER_VIRTUAL => Ok((Clock::ProcessCputimeId, Signal::SIGVTALRM)),
			ITIMER_PROF => Ok((Clock::ProcessCputimeId, Signal::SIGPROF)),
			_ => Err(errno!(EINVAL)),
		}
	}

	/// Returns the current state of the current process's interval timer `which` as
	/// `(interval_ns, value_ns)`, where `value_ns` is the time remaining until the next firing
	/// (`0` if disarmed).
	///
	/// If `which` is not a valid interval timer, the function returns `EINVAL`.
	pub fn get_itimer(which: c_int) -> EResult<(Timestamp, Timestamp)> {
		Self::itimer_clock_signal(which)?;
		let proc = Process::current();
		let this = proc.timer_manager.lock();
		Ok(this.itimers[which as usize]
			.as_ref()
			.map(Timer::get_time)
			.unwrap_or((0, 0)))
	}

	/// Sets the state of the current process's interval timer `which`.
	///
	/// Arguments:
	/// - `which` is the timer to set (`ITIMER_REAL`, `ITIMER_VIRTUAL` or `ITIMER_PROF`)
	/// - `interval` is the interval between two firings, in nanoseconds
	/// - `value` is the time until the next firing, in nanoseconds (`0` disarms the timer)
	///
	/// On success, the function returns the previous state as `(interval_ns, value_ns)`.
	///
	/// If `which` is not a valid interval timer, the function returns `EINVAL`.
	pub fn set_itimer(
		which: c_int,
		interval: Timestamp,
		value: Timestamp,
	) -> EResult<(Timestamp, Timestamp)> {
		// Validate `which` (returns `EINVAL` for an invalid value)
		let (clock, sig) = Self::itimer_clock_signal(which)?;
		// TODO implement `Clock::ProcessCputimeId` accounting and arm these for real.
		if which != ITIMER_REAL {
			return Ok((0, 0));
		}
		let proc = Process::current();
		let mut this = proc.timer_manager.lock();
		let slot = &mut this.itimers[which as usize];
		// Read the previous state before mutating
		let old = slot.as_ref().map(Timer::get_time).unwrap_or((0, 0));
		// Lazily create the timer on first use
		if slot.is_none() {
			let proc = proc.clone();
			*slot = Some(Timer::new(clock, move || Process::kill(&proc, sig))?);
		}
		slot.as_mut().unwrap().set_time(interval, value)?;
		Ok(old)
	}
}

// TODO use intrusive binary trees in order to avoid memory allocations
/// The queue of timers to be fired next.
///
/// The key has the following elements:
/// - the timestamp, in nanoseconds, at which the timer will fire next
/// - a pointer to the timer
static TIMERS_QUEUE: IntSpin<BTreeMap<(Timestamp, *const TimerInner), ()>> =
	IntSpin::new(BTreeMap::new());

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
		(timer.f)();
		if timer.is_oneshot() {
			queue.pop_first();
		} else {
			oom::wrap(|| timer.reset(&mut queue, ts));
		}
	}
}
