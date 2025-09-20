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

//! Mutually exclusive access primitive based on a spinlock.
//!
//! A [`Spin`] protects its wrapped data from being accessed concurrently, avoid data races.
//!
//! One particularity with kernel development is that multi-threading is not the
//! only way to get concurrency issues. An interruption may be triggered at any moment.
//!
//! For this reason, spinlocks in the kernel are equipped with an option allowing to disable
//! non-maskable interrupts while being locked.

use crate::arch::{
	x86,
	x86::{cli, sti},
};
use core::{
	cell::UnsafeCell,
	fmt::{self, Formatter},
	hint,
	ops::{Deref, DerefMut},
	sync::atomic::{
		AtomicBool,
		Ordering::{Acquire, Release},
	},
};

#[inline(always)]
fn lock(lock: &AtomicBool) {
	while lock.swap(true, Acquire) {
		hint::spin_loop();
	}
}

/// Unlocks the associated [`Spin`] when dropped.
pub struct SpinGuard<'m, T: ?Sized, const INT: bool> {
	spin: &'m Spin<T, INT>,
	/// The interrupt status before locking. This field is relevant only if `INT == false`
	int_state: bool,
}

impl<T: ?Sized, const INT: bool> Deref for SpinGuard<'_, T, INT> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.spin.data.get() }
	}
}

impl<T: ?Sized, const INT: bool> DerefMut for SpinGuard<'_, T, INT> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.spin.data.get() }
	}
}

impl<T: ?Sized, const INT: bool> !Send for SpinGuard<'_, T, INT> {}

unsafe impl<T: ?Sized + Sync, const INT: bool> Sync for SpinGuard<'_, T, INT> {}

impl<T: ?Sized + fmt::Debug, const INT: bool> fmt::Debug for SpinGuard<'_, T, INT> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self.deref(), f)
	}
}

impl<T: ?Sized, const INT: bool> Drop for SpinGuard<'_, T, INT> {
	fn drop(&mut self) {
		unsafe {
			self.spin.unlock(self.int_state);
		}
	}
}

/// Wraps a value which be accessed by only one context at a time.
///
/// The `INT` generic parameter tells whether interrupts are allowed while locked. The default
/// value is `true`.
pub struct Spin<T: ?Sized, const INT: bool = true> {
	spin: AtomicBool,
	data: UnsafeCell<T>,
}

impl<T, const INT: bool> Spin<T, INT> {
	/// Creates a new instance wrapping the given `data`.
	pub const fn new(data: T) -> Self {
		Self {
			spin: AtomicBool::new(false),
			data: UnsafeCell::new(data),
		}
	}
}

impl<T: Default, const INT: bool> Default for Spin<T, INT> {
	fn default() -> Self {
		Self::new(Default::default())
	}
}

impl<T: ?Sized, const INT: bool> Spin<T, INT> {
	/// Acquires the spinlock.
	///
	/// If the spinlock is already acquired, the thread loops until it becomes available.
	///
	/// The function returns a [`SpinGuard`] associated with `self`. When dropped, the spinlock
	/// is unlocked.
	pub fn lock(&self) -> SpinGuard<T, INT> {
		let int_state = if !INT {
			let enabled = x86::is_interrupt_enabled();
			cli();
			enabled
		} else {
			// In this case, this value does not matter
			false
		};
		lock(&self.spin);
		SpinGuard {
			spin: self,
			int_state,
		}
	}

	/// Releases the spinlock. This function should not be used directly since it is called when
	/// the guard is dropped.
	///
	/// `int_state` is the state of interruptions before locking.
	///
	/// # Safety
	///
	/// If the spinlock is not locked, the behaviour is undefined.
	///
	/// Releasing while the resource is being used may result in concurrent accesses.
	pub unsafe fn unlock(&self, int_state: bool) {
		self.spin.store(false, Release);
		if !INT && int_state {
			sti();
		}
	}
}

impl<T, const INT: bool> Spin<T, INT> {
	/// Acquires the spinlock, consumes it and returns the inner value.
	///
	/// The function does not disable nor enable interruptions.
	pub fn into_inner(self) -> T {
		lock(&self.spin);
		self.data.into_inner()
	}
}

unsafe impl<T, const INT: bool> Sync for Spin<T, INT> {}

impl<T: ?Sized + fmt::Debug, const INT: bool> fmt::Debug for Spin<T, INT> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let guard = self.lock();
		fmt::Debug::fmt(&*guard, f)
	}
}

/// Type alias on [`Spin`] representing a spinlock which masks interrupts.
pub type IntSpin<T> = Spin<T, false>;
/// Type alias on [`SpinGuard`] representing a spinlock guard which masks interrupts.
pub type IntSpinGuard<'m, T> = SpinGuard<'m, T, false>;
