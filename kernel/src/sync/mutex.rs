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

//! Sleeping mutual exclusion synchronization primitive.
//!
//! Contrary to a spinlock, [`Mutex`] makes the current task sleep while waiting, reducing CPU
//! cycles waste.

use crate::{
	process,
	process::{Process, State, scheduler::schedule},
	sync::spin::IntSpin,
};
use core::{
	cell::UnsafeCell,
	fmt,
	fmt::Formatter,
	ops::{Deref, DerefMut},
};
use utils::{errno, errno::EResult, list, list_type};

fn lock<const INT: bool>(queue: &IntSpin<Queue>) -> EResult<()> {
	{
		let mut q = queue.lock();
		q.acquired += 1;
		// If no one else has acquired the mutex, return
		if q.acquired == 1 {
			return Ok(());
		}
		// At least one other task has acquired the mutex: we must sleep. The process is dequeued
		// when the mutex is released by the previous task that acquired it
		q.wait_queue.insert_back(Process::current());
		// Put to sleep before releasing the spinlock to make sure another process releasing
		// the mutex does not try to wake us up before we sleep
		if INT {
			process::set_state(State::IntSleeping);
		} else {
			process::set_state(State::Sleeping);
		}
	}
	schedule();
	let proc = Process::current();
	// Make sure the process is dequeued
	unsafe {
		queue.lock().wait_queue.remove(&proc);
	}
	// If woken up by a signal
	if INT && proc.has_pending_signal() {
		return Err(errno!(EINTR));
	}
	Ok(())
}

/// Unlocks the associated [`Mutex`] when dropped.
pub struct MutexGuard<'m, T: ?Sized, const INT: bool> {
	mutex: &'m Mutex<T, INT>,
}

impl<T: ?Sized, const INT: bool> Deref for MutexGuard<'_, T, INT> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.mutex.data.get() }
	}
}

impl<T: ?Sized, const INT: bool> DerefMut for MutexGuard<'_, T, INT> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.mutex.data.get() }
	}
}

impl<T: ?Sized, const INT: bool> !Send for MutexGuard<'_, T, INT> {}

unsafe impl<T: ?Sized + Sync, const INT: bool> Sync for MutexGuard<'_, T, INT> {}

impl<T: ?Sized + fmt::Debug, const INT: bool> fmt::Debug for MutexGuard<'_, T, INT> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self.deref(), f)
	}
}

impl<T: ?Sized, const INT: bool> Drop for MutexGuard<'_, T, INT> {
	fn drop(&mut self) {
		unsafe {
			self.mutex.unlock();
		}
	}
}

struct Queue {
	acquired: usize,
	wait_queue: list_type!(Process, wait_queue),
}

/// Sleeping mutex.
///
/// `INT` tells whether sleeping can be interrupted by a signal.
pub struct Mutex<T: ?Sized, const INT: bool> {
	queue: IntSpin<Queue>,
	data: UnsafeCell<T>,
}

impl<T: Default, const INT: bool> Default for Mutex<T, INT> {
	fn default() -> Self {
		Self::new(Default::default())
	}
}

impl<T, const INT: bool> Mutex<T, INT> {
	/// Creates a new instance wrapping the given `data`.
	pub const fn new(data: T) -> Self {
		Self {
			queue: IntSpin::new(Queue {
				acquired: 0,
				wait_queue: list!(Process, wait_queue),
			}),
			data: UnsafeCell::new(data),
		}
	}
}

impl<T> Mutex<T, false> {
	/// Acquires the mutex, consumes it and returns the inner value.
	pub fn into_inner(self) -> T {
		let _ = lock::<false>(&self.queue);
		self.data.into_inner()
	}
}

impl<T: ?Sized, const INT: bool> Mutex<T, INT> {
	/// Releases the mutex, waking up the next process waiting on it, if any.
	///
	/// # Safety
	///
	/// This function should not be used directly since it is called when the guard is dropped.
	///
	/// If the mutex is not locked, the behaviour is undefined.
	///
	/// Releasing while the resource is being used is undefined.
	pub unsafe fn unlock(&self) {
		let next = {
			let mut q = self.queue.lock();
			q.acquired -= 1;
			// If at least one other task is waiting, wake it up
			q.wait_queue.remove_front()
		};
		if let Some(next) = next {
			let mut mask = State::Sleeping as u8;
			if INT {
				mask |= State::IntSleeping as u8;
			}
			Process::wake_from(&next, mask);
		}
	}
}

impl<T: ?Sized> Mutex<T, false> {
	/// Acquires the mutex.
	///
	/// If the mutex is already acquired, the thread loops until it becomes available.
	///
	/// The function returns a [`MutexGuard`] associated with `self`. When dropped, the mutex
	/// is unlocked.
	pub fn lock(&self) -> MutexGuard<T, false> {
		let _ = lock::<false>(&self.queue);
		MutexGuard {
			mutex: self,
		}
	}
}

impl<T: ?Sized> Mutex<T, true> {
	/// Acquires the mutex.
	///
	/// If the mutex is already acquired, the thread loops until it becomes available.
	///
	/// The function returns a [`MutexGuard`] associated with `self`. When dropped, the mutex
	/// is unlocked.
	///
	/// If the current process is interrupted by a signal while waiting, the function returns with
	/// the errno [`EINTR`].
	pub fn lock(&self) -> EResult<MutexGuard<T, true>> {
		lock::<true>(&self.queue)?;
		Ok(MutexGuard {
			mutex: self,
		})
	}
}

unsafe impl<T, const INT: bool> Sync for Mutex<T, INT> {}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T, false> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let guard = self.lock();
		fmt::Debug::fmt(&*guard, f)
	}
}
