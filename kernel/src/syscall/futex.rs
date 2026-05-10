/*
 * Copyright 2026 Luc Lenôtre
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

//! The `futex` system call provides fast userspace mutual exclusion primitives.

use crate::{
	memory::user::UserPtr,
	process::{Process, State},
	sync::{spin::Spin, wait_queue::WaitQueue},
	time::{
		clock::{Clock, current_time_ns},
		timer::Timer,
		unit::{TimeUnit, Timespec, Timespec32, Timestamp},
	},
};
use core::{ffi::c_int, hint::unlikely, ptr::NonNull};
use utils::{collections::hashmap::HashMap, errno, errno::EResult, ptr::arc::Arc};

/// Wait if `*uaddr == val`.
const FUTEX_WAIT: c_int = 0;
/// Wake up to `val` waiters on `uaddr`.
const FUTEX_WAKE: c_int = 1;
/// Like [`FUTEX_WAIT`] but with an absolute timeout and a 32-bit bitset filter.
const FUTEX_WAIT_BITSET: c_int = 9;
/// Like [`FUTEX_WAKE`] but with a 32-bit bitset filter.
const FUTEX_WAKE_BITSET: c_int = 10;

/// Restricts the futex to the calling process.
const FUTEX_PRIVATE_FLAG: c_int = 128;
/// Use [`Clock::Realtime`] instead of [`Clock::Monotonic`] for the timeout.
const FUTEX_CLOCK_REALTIME: c_int = 256;

const FUTEX_CMD_MASK: c_int = !(FUTEX_PRIVATE_FLAG | FUTEX_CLOCK_REALTIME);

/// Identifies a futex word.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct FutexKey {
	/// Raw pointer of the [`crate::process::mem_space::MemSpace`] holding the address.
	mem_space: usize,
	/// The virtual address of the futex word.
	addr: usize,
}

/// Map from futex words to wait queues.
///
/// Entries are reclaimed by [`cleanup_if_unused`] when no waiters remain and the only
/// outstanding [`Arc`] references are the map's and the caller's.
static FUTEXES: Spin<HashMap<FutexKey, Arc<WaitQueue>>> = Spin::new(HashMap::new());

fn make_key(addr: usize) -> FutexKey {
	let mem_space = Arc::as_ptr(Process::current().mem_space()) as usize;
	FutexKey {
		mem_space,
		addr,
	}
}

fn lookup(key: &FutexKey) -> Option<Arc<WaitQueue>> {
	FUTEXES.lock().get(key).cloned()
}

fn lookup_or_create(key: FutexKey) -> EResult<Arc<WaitQueue>> {
	let mut map = FUTEXES.lock();
	if let Some(q) = map.get(&key) {
		return Ok(q.clone());
	}
	let q = Arc::new(WaitQueue::new())?;
	map.insert(key, q.clone())?;
	Ok(q)
}

/// Removes the map entry for `key` if the queue has no waiters
fn cleanup_if_unused(key: &FutexKey, queue: &Arc<WaitQueue>) {
	let mut map = FUTEXES.lock();
	if Arc::strong_count(queue) > 2 || !queue.is_empty() {
		return;
	}
	// Only remove if the map still points at *our* queue
	if let Some(stored) = map.get(key)
		&& Arc::as_ptr(stored) == Arc::as_ptr(queue)
	{
		map.remove(key);
	}
}

/// Validates that `uaddr` is non-null and 4-byte aligned, returning a [`UserPtr`] on it.
fn user_word(uaddr: *mut u32) -> EResult<UserPtr<u32>> {
	if unlikely(uaddr.is_null() || (uaddr as usize) % 4 != 0) {
		return Err(errno!(EINVAL));
	}
	Ok(UserPtr(NonNull::new(uaddr)))
}

/// Performs `FUTEX_WAIT` / `FUTEX_WAIT_BITSET`.
///
/// `delay` is the relative timeout, in nanoseconds. `0` means "no timeout".
fn do_wait(uaddr: *mut u32, val: u32, clock: Clock, delay: Timestamp) -> EResult<()> {
	let user = user_word(uaddr)?;
	let key = make_key(uaddr as usize);
	let queue = lookup_or_create(key)?;
	// Set up a timer if a timeout was given. Dropping the timer at the end of the function
	// removes it from the timer queue.
	let _timer = if delay > 0 {
		let proc = Process::current();
		let mut t = Timer::new(clock, move || {
			Process::wake_from(&proc, State::IntSleeping as u8);
		})?;
		t.set_time(0, delay)?;
		Some(t)
	} else {
		None
	};
	let deadline = if delay > 0 {
		Some(current_time_ns(clock).saturating_add(delay))
	} else {
		None
	};
	let res = queue.wait_check(|| {
		let cur = user.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
		if cur != val {
			return Err(errno!(EAGAIN));
		}
		Ok(())
	});
	cleanup_if_unused(&key, &queue);
	// Convert a normal wake into `ETIMEDOUT` if the timer has expired
	if res.is_ok()
		&& let Some(deadline) = deadline
		&& current_time_ns(clock) >= deadline
	{
		return Err(errno!(ETIMEDOUT));
	}
	res
}

/// Performs `FUTEX_WAKE` / `FUTEX_WAKE_BITSET`.
fn do_wake(uaddr: *mut u32, val: u32) -> EResult<usize> {
	user_word(uaddr)?;
	let key = make_key(uaddr as usize);
	let Some(queue) = lookup(&key) else {
		return Ok(0);
	};
	let woken = queue.wake_n(val as usize);
	cleanup_if_unused(&key, &queue);
	Ok(woken)
}

/// Common dispatch for `futex`, parameterized on the timespec ABI.
///
/// `timeout_ns` returns the timespec at the given userspace pointer in nanoseconds.
fn do_futex(
	uaddr: *mut u32,
	op: c_int,
	val: u32,
	timeout_ns: impl FnOnce() -> EResult<Timestamp>,
) -> EResult<usize> {
	let cmd = op & FUTEX_CMD_MASK;
	let clock = if op & FUTEX_CLOCK_REALTIME != 0 {
		Clock::Realtime
	} else {
		Clock::Monotonic
	};
	match cmd {
		FUTEX_WAIT => {
			let delay = timeout_ns()?;
			do_wait(uaddr, val, Clock::Monotonic, delay)?;
			Ok(0)
		}
		FUTEX_WAIT_BITSET => {
			let delay = match timeout_ns()? {
				0 => 0,
				ts => {
					let now = current_time_ns(clock);
					if ts <= now {
						return Err(errno!(ETIMEDOUT));
					}
					ts - now
				}
			};
			do_wait(uaddr, val, clock, delay)?;
			Ok(0)
		}
		FUTEX_WAKE | FUTEX_WAKE_BITSET => do_wake(uaddr, val),
		_ => Err(errno!(ENOSYS)),
	}
}

/// 32-bit ABI: `timeout` points to a [`Timespec32`].
pub fn futex(
	uaddr: *mut u32,
	op: c_int,
	val: u32,
	timeout: UserPtr<Timespec32>,
	_uaddr2: *mut u32,
	_val3: u32,
) -> EResult<usize> {
	do_futex(uaddr, op, val, || {
		Ok(timeout
			.copy_from_user()?
			.map(|ts| ts.to_nano())
			.unwrap_or(0))
	})
}

/// 64-bit ABI: `timeout` points to a [`Timespec`].
pub fn futex_time64(
	uaddr: *mut u32,
	op: c_int,
	val: u32,
	timeout: UserPtr<Timespec>,
	_uaddr2: *mut u32,
	_val3: u32,
) -> EResult<usize> {
	do_futex(uaddr, op, val, || {
		Ok(timeout
			.copy_from_user()?
			.map(|ts| ts.to_nano())
			.unwrap_or(0))
	})
}
