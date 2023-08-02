//! This module implements timers.

use super::clock;
use super::unit::ClockIdT;
use super::unit::ITimerspec32;
use super::unit::TimeUnit;
use super::unit::TimerT;
use super::unit::Timespec;
use super::unit::TimestampScale;
use crate::errno::EResult;
use crate::errno::Errno;
use crate::limits;
use crate::process::oom;
use crate::process::pid::Pid;
use crate::process::signal::SigEvent;
use crate::process::signal::Signal;
use crate::process::signal::SIGEV_SIGNAL;
use crate::process::signal::SIGEV_THREAD;
use crate::process::Process;
use crate::time::unit::Timespec32;
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

	/// The timer's interval between firing.
	interval: Timespec32,
	/// The next timestamp at which the timer will expire. If `None`, the timer is unarmed.
	next: Option<Timespec>,
}

impl Timer {
	/// Creates a timer.
	///
	/// Arguments:
	/// - `clockid` is the ID of the clock to use.
	/// - `sevp` describes the event to be triggered by the clock.
	pub fn new(clockid: ClockIdT, sevp: SigEvent) -> EResult<Self> {
		// Check arguments are valid
		let _ = clock::current_time(clockid, TimestampScale::Nanosecond)?;
		if !sevp.is_valid() {
			return Err(errno!(EINVAL));
		}

		Ok(Self {
			clockid,
			sevp,

			interval: Default::default(),
			next: Default::default(),
		})
	}

	/// Tells whether the timer is armed.
	#[inline]
	pub fn is_armed(&self) -> bool {
		self.next.is_some()
	}

	/// Tells whether the timer must be fired.
	///
	/// `curr` is the current timestamp.
	#[inline]
	pub fn has_expired(&self, curr: &Timespec) -> bool {
		self.next.as_ref().map(|next| curr >= next).unwrap_or(true)
	}

	/// Tells whether the timer is oneshot. If not, the timer repeats until manually stopped.
	#[inline]
	pub fn is_oneshot(&self) -> bool {
		self.interval.is_zero()
	}

	/// Returns the current state of the timer.
	#[inline]
	pub fn get_time(&self) -> ITimerspec32 {
		let ts: Timespec = clock::current_time_struct(self.clockid).unwrap();
		let value = self.next.map(|next| next - ts).unwrap_or_default();

		ITimerspec32 {
			it_interval: self.interval,
			it_value: Timespec32 {
				tv_nsec: value.tv_nsec as _,
				tv_sec: value.tv_sec as _,
			},
		}
	}

	/// Computes the timestamp at which the timer will fire next.
	///
	/// Arguments:
	/// - `spec` is the timer's setting.
	/// - `ts` is the current timestamp.
	fn compute_next(spec: &ITimerspec32, ts: Timespec32) -> Timespec {
		let time = if spec.it_value.is_zero() {
			spec.it_interval
		} else {
			ts + spec.it_value
		};

		Timespec {
			tv_nsec: time.tv_nsec as _,
			tv_sec: time.tv_sec as _,
		}
	}

	/// Sets the timer's state.
	///
	/// Arguments:
	/// - `spec` is the new setting of the timer.
	/// - `pid` is the PID of the process associated with the timer.
	/// - `timer_id` is the ID of the timer.
	///
	/// On allocation error, the function returns an error.
	#[inline]
	pub fn set_time(&mut self, spec: ITimerspec32, pid: Pid, timer_id: TimerT) -> EResult<()> {
		let mut queue = TIMERS_QUEUE.lock();
		if let Some(next) = self.next {
			queue.remove(&(next, pid, timer_id));
		}

		let ts: Timespec32 = clock::current_time_struct(self.clockid).unwrap();
		let next = Self::compute_next(&spec, ts);
		self.interval = spec.it_interval;
		self.next = Some(next);

		queue.insert((next, pid, timer_id), ())?;
		Ok(())
	}

	/// Fires the timer.
	///
	/// `proc` is the process to which the timer is fired.
	pub fn fire(&mut self, proc: &mut Process) {
		match self.sevp.sigev_notify {
			SIGEV_SIGNAL => {
				let Ok(signal) = Signal::try_from(self.sevp.sigev_signo as u32) else {
                    return;
                };

				// TODO on sigint_t, set si_code to SI_TIMER
				proc.kill(&signal, false);
			}

			SIGEV_THREAD => todo!(), // TODO

			_ => {}
		}
	}

	/// Resets the timer to be fired again.
	///
	/// Arguments:
	/// - `queue` is the queue.
	/// - `ts` is the timestamp.
	/// - `pid` is the PID of the process associated with the timer.
	/// - `timer_id` is the ID of the timer.
	///
	/// On allocation error, the function returns an error.
	fn reset(
		&mut self,
		queue: &mut Map<(Timespec, Pid, TimerT), ()>,
		ts: Timespec,
		pid: Pid,
		timer_id: TimerT,
	) -> EResult<()> {
		if let Some(next) = self.next {
			queue.remove(&(next, pid, timer_id));
		}

		if self.interval.is_zero() {
			self.next = None;
			return Ok(());
		}

		let next = Self::compute_next(
			&ITimerspec32 {
				it_interval: self.interval,
				it_value: self.interval,
			},
			Timespec32 {
				tv_nsec: ts.tv_nsec as _,
				tv_sec: ts.tv_sec as _,
			},
		);
		queue.insert((next, pid, timer_id), ())?;

		self.next = Some(next);

		Ok(())
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
		let timer = Timer::new(clockid, sevp)?;
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
		queue.retain(|(_, pid, _), _| *pid != self.pid);
	}
}

/// The queue of timers to be fired next.
///
/// The key has the following elements:
/// - the timestamp at which the timer will fire next
/// - the PID of the process owning the timer
/// - the ID of the timer
static TIMERS_QUEUE: IntMutex<Map<(Timespec, Pid, TimerT), ()>> = IntMutex::new(Map::new());

/// Ticks active timers and triggers them if necessary.
pub(super) fn tick() {
	let mut times: [Option<Timespec>; 12] = [None; 12];
	let mut queue = TIMERS_QUEUE.lock();

	loop {
		// Peek next timer
		let Some(((_, pid, timer_id), _)) = queue.first_key_value() else {
            break;
        };
		let pid = *pid;
		let timer_id = *timer_id;

		// Get process
		let Some(proc_mutex) = Process::get_by_pid(pid) else {
            // invalid timer, remove
			queue.pop_first();
            break;
        };
		let mut proc = proc_mutex.lock();
		// Get timer manager
		let timer_manager_mutex = proc.timer_manager();
		let mut timer_manager = timer_manager_mutex.lock();

		// Get timer
		let Some(timer) = timer_manager.get_timer_mut(timer_id) else {
            // invalid timer, remove
			queue.pop_first();
            break;
        };

		// Get current time
		let ts = match times[timer.clockid as usize] {
			Some(ts) => ts,
			None => {
				let ts = clock::current_time_struct(timer.clockid).unwrap();
				times[timer.clockid as usize] = Some(ts);
				ts
			}
		};

		if !timer.has_expired(&ts) {
			// If this timer has not expired, all the next timers won't be expired either
			break;
		}

		timer.fire(&mut proc);

		if timer.is_oneshot() {
			queue.pop_first();
		} else {
			oom::wrap(|| timer.reset(&mut queue, ts, pid, timer_id));
		}
	}
}
