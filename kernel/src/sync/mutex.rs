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
	process::{State, scheduler::schedule},
};
use core::{
	cell::UnsafeCell,
	fmt,
	fmt::Formatter,
	hint,
	ops::{Deref, DerefMut},
	sync::atomic::{
		AtomicBool,
		Ordering::{Acquire, Release},
	},
};

// TODO insert the task in a queue before sleeping so that it can get woken up
fn lock(spin: &AtomicBool) {
	// Fast path
	for _ in 0..100 {
		if !spin.swap(true, Acquire) {
			return;
		}
		hint::spin_loop();
	}
	// Slow path
	while spin.swap(true, Acquire) {
		// Sleep
		process::set_state(State::Sleeping);
		// If unlocked in between, cancel sleeping
		if !spin.swap(true, Acquire) {
			process::set_state(State::Running);
			return;
		}
		// Wait until woken up
		schedule();
	}
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

/// Sleeping mutex.
pub struct Mutex<T: ?Sized> {
	spin: AtomicBool,
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
			spin: AtomicBool::new(false),
			data: UnsafeCell::new(data),
		}
	}

	/// Acquires the mutex, consumes it and returns the inner value.
	pub fn into_inner(self) -> T {
		lock(&self.spin);
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
		lock(&self.spin);
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
	#[inline]
	pub unsafe fn unlock(&self) {
		self.spin.store(false, Release);
		// TODO wake up a queued task
	}
}

unsafe impl<T> Sync for Mutex<T> {}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let guard = self.lock();
		fmt::Debug::fmt(&*guard, f)
	}
}
