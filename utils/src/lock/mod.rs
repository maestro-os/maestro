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

//! Mutually exclusive access primitive implementation.
//!
//! A `Mutex` allows to ensure that one, and only thread accesses its data at once, preventing race
//! conditions.
//!
//! One particularity with kernel development is that multi-threading is not the
//! only way to get concurrency issues. Another factor to take into account is
//! that fact that an interruption may be triggered at any moment while
//! executing the code unless disabled.
//!
//! For this reason, mutexes in the kernel are equipped with an option allowing to disable
//! interrupts while being locked.
//!
//! If an exception is raised while a mutex that disables interruptions is
//! acquired, the behaviour is undefined.

pub mod atomic;
pub mod once;
pub mod spinlock;

use crate::{
	interrupt,
	interrupt::{cli, sti},
	lock::spinlock::Spinlock,
};
use core::{
	cell::UnsafeCell,
	fmt::{self, Formatter},
	ops::{Deref, DerefMut},
};

/// Type used to declare a guard meant to unlock the associated `Mutex` at the
/// moment the execution gets out of the scope of its declaration.
pub struct MutexGuard<'m, T: ?Sized, const INT: bool> {
	/// The locked mutex.
	mutex: &'m Mutex<T, INT>,
	/// The interrupt status before locking. This field is relevant only if `INT == false`.
	int_state: bool,
}

impl<T: ?Sized, const INT: bool> Deref for MutexGuard<'_, T, INT> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &(*self.mutex.inner.get()).data }
	}
}

impl<T: ?Sized, const INT: bool> DerefMut for MutexGuard<'_, T, INT> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut (*self.mutex.inner.get()).data }
	}
}

unsafe impl<T: ?Sized + Sync, const INT: bool> Sync for MutexGuard<'_, T, INT> {}

impl<T: ?Sized + fmt::Debug, const INT: bool> fmt::Debug for MutexGuard<'_, T, INT> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self.deref(), f)
	}
}

impl<T: ?Sized, const INT: bool> Drop for MutexGuard<'_, T, INT> {
	fn drop(&mut self) {
		unsafe {
			self.mutex.unlock(self.int_state);
		}
	}
}

/// The inner structure of [`Mutex`].
struct MutexIn<T: ?Sized, const INT: bool> {
	/// The spinlock for the underlying data.
	spin: Spinlock,
	/// The data associated to the mutex.
	data: T,
}

/// The object wrapped in a `Mutex` can be accessed by only one thread at a time.
///
/// The `INT` generic parameter tells whether interrupts are allowed while
/// the mutex is locked. The default value is `true`.
pub struct Mutex<T: ?Sized, const INT: bool = true> {
	/// An unsafe cell to the inner structure of the Mutex.
	inner: UnsafeCell<MutexIn<T, INT>>,
}

impl<T, const INT: bool> Mutex<T, INT> {
	/// Creates a new Mutex with the given data to be owned.
	pub const fn new(data: T) -> Self {
		Self {
			inner: UnsafeCell::new(MutexIn {
				spin: Spinlock::new(),
				data,
			}),
		}
	}
}

impl<T: Default, const INT: bool> Default for Mutex<T, INT> {
	fn default() -> Self {
		Self::new(Default::default())
	}
}

impl<T: ?Sized, const INT: bool> Mutex<T, INT> {
	/// Locks the mutex.
	///
	/// If the mutex is already locked, the thread shall wait until it becomes available.
	///
	/// The function returns a [`MutexGuard`] associated with `self`. When dropped, the mutex is
	/// unlocked.
	pub fn lock(&self) -> MutexGuard<T, INT> {
		let int_state = if !INT {
			let enabled = interrupt::is_enabled();
			cli();
			enabled
		} else {
			// In this case, this value does not matter
			false
		};
		// Safe because using the spinlock
		let inner = unsafe { &mut *self.inner.get() };
		inner.spin.lock();
		MutexGuard {
			mutex: self,
			int_state,
		}
	}

	/// Unlocks the mutex. This function should not be used directly since it is called when the
	/// mutex guard is dropped.
	///
	/// `int_state` is the state of interruptions before locking.
	///
	/// # Safety
	///
	/// If the mutex is not locked, the behaviour is undefined.
	///
	/// Unlocking the mutex while the resource is being used may result in concurrent accesses.
	pub unsafe fn unlock(&self, int_state: bool) {
		let inner = &mut (*self.inner.get());
		inner.spin.unlock();
		if !INT && int_state {
			sti();
		}
	}
}

impl<T, const INT: bool> Mutex<T, INT> {
	/// Locks the mutex, consumes it and returns the inner value.
	///
	/// If the mutex disables interruptions, it is the caller's responsibility to handle it
	/// afterward.
	pub fn into_inner(self) -> T {
		// Make sure no one is using the resource
		let inner = unsafe { &mut *self.inner.get() };
		inner.spin.lock();
		self.inner.into_inner().data
	}
}

unsafe impl<T, const INT: bool> Sync for Mutex<T, INT> {}

impl<T: ?Sized + fmt::Debug, const INT: bool> fmt::Debug for Mutex<T, INT> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let guard = self.lock();
		fmt::Debug::fmt(&*guard, f)
	}
}

/// Type alias on [`Mutex`] representing a mutex which masks interrupts.
pub type IntMutex<T> = Mutex<T, false>;
/// Type alias on [`MutexGuard`] representing a mutex which masks interrupts.
pub type IntMutexGuard<'m, T> = MutexGuard<'m, T, false>;
