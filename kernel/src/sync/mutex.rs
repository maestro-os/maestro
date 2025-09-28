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
use utils::{list, list_type};

fn lock(queue: &IntSpin<Queue>) {
	{
		let mut q = queue.lock();
		q.acquired += 1;
		// If no one else has acquired the mutex, return
		if q.acquired == 1 {
			return;
		}
		// At least one other task has acquired the mutex: we must sleep. The process is dequeued
		// when the mutex is released by the previous task that acquired it
		q.wait_queue.insert_back(Process::current());
		// Put to sleep before releasing the spinlock to make sure another process releasing
		// the mutex does not try to wake us up before we sleep
		process::set_state(State::Sleeping);
	}
	schedule();
	// TODO if interruptible sleep, and woken up by a signal, remove from the queue and return
	// EINTR
}

/// Unlocks the associated [`Mutex`] when dropped.
pub struct MutexGuard<'m, T: ?Sized> {
	mutex: &'m Mutex<T>,
}

impl<T: ?Sized> Deref for MutexGuard<'_, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.mutex.data.get() }
	}
}

impl<T: ?Sized> DerefMut for MutexGuard<'_, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.mutex.data.get() }
	}
}

impl<T: ?Sized> !Send for MutexGuard<'_, T> {}

unsafe impl<T: ?Sized + Sync> Sync for MutexGuard<'_, T> {}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'_, T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self.deref(), f)
	}
}

impl<T: ?Sized> Drop for MutexGuard<'_, T> {
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
pub struct Mutex<T: ?Sized> {
	queue: IntSpin<Queue>,
	data: UnsafeCell<T>,
}

impl<T: Default> Default for Mutex<T> {
	fn default() -> Self {
		Self::new(Default::default())
	}
}

impl<T> Mutex<T> {
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

	/// Acquires the mutex, consumes it and returns the inner value.
	pub fn into_inner(self) -> T {
		lock(&self.queue);
		self.data.into_inner()
	}
}

impl<T: ?Sized> Mutex<T> {
	/// Acquires the mutex.
	///
	/// If the mutex is already acquired, the thread loops until it becomes available.
	///
	/// The function returns a [`MutexGuard`] associated with `self`. When dropped, the mutex
	/// is unlocked.
	pub fn lock(&self) -> MutexGuard<T> {
		lock(&self.queue);
		MutexGuard {
			mutex: self,
		}
	}

	/// Releases the mutex. This function should not be used directly since it is called when
	/// the guard is dropped.
	///
	/// # Safety
	///
	/// Releasing while the resource is being used is undefined.
	pub unsafe fn unlock(&self) {
		let next = {
			let mut q = self.queue.lock();
			debug_assert_ne!(q.acquired, 0);
			q.acquired -= 1;
			// If at least one other task is waiting, wake it up
			q.wait_queue.remove_front()
		};
		if let Some(next) = next {
			Process::wake_from(&next, State::Sleeping as u8);
		}
	}
}

unsafe impl<T> Sync for Mutex<T> {}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let guard = self.lock();
		fmt::Debug::fmt(&*guard, f)
	}
}
