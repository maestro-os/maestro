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

//! Once-initialized objects.

use core::{cell::UnsafeCell, mem::MaybeUninit, ops::Deref};

/// An object that is meant to be initialized once at boot, then accessed in read-only.
///
/// The value **must** be initialized with `init` before calling `get`. Failure to do so results in
/// an undefined behavior.
pub struct OnceInit<T>(UnsafeCell<MaybeUninit<T>>);

impl<T> OnceInit<T> {
	/// Creates a new instance waiting to be initialized.
	///
	/// # Safety
	///
	/// The value **must** be initialized with before calling `get`.
	pub const unsafe fn new() -> Self {
		Self(UnsafeCell::new(MaybeUninit::uninit()))
	}

	/// Initializes with the given value.
	///
	/// If already initialized, the previous value is **not** dropped.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to enforce concurrency rules.
	pub unsafe fn init(this: &Self, val: T) -> &T {
		unsafe {
			let inner = &mut *this.0.get();
			inner.write(val);
			inner.assume_init_ref()
		}
	}
}

impl<T> Deref for OnceInit<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { (*self.0.get()).assume_init_ref() }
	}
}

unsafe impl<T> Sync for OnceInit<T> {}
