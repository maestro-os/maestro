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

//! This module contains the Spinlock structure, which is considered as being a
//! low level feature.
//!
//! Unless for special cases, other locks should be used instead.

use core::{
	hint,
	sync::atomic::{AtomicBool, Ordering},
};

/// A spinlock is a lock that is used to prevent a specific piece of code from
/// being accessed by more than one thread at a time.
///
/// It works by storing a value telling whether a thread is already in that piece of code.
///
/// To avoid race conditions, the implementation uses an atomic exchange instruction to
/// check/lock the structure. If a threads tries to lock the structure while
/// already being locked, the thread shall wait in a loop (spin) until the
/// structure is unlocked.
///
/// Special attention must be aimed toward the usage of this structure since it
/// can easily result in deadlocks if misused.
pub struct Spinlock {
	locked: AtomicBool,
}

impl Spinlock {
	/// Creates a new spinlock.
	pub const fn new() -> Self {
		Self {
			locked: AtomicBool::new(false),
		}
	}

	/// Locks the spinlock.
	#[inline(always)]
	pub fn lock(&mut self) {
		while self.locked.swap(true, Ordering::Acquire) {
			hint::spin_loop();
		}
	}

	/// Unlocks the spinlock.
	///
	/// # Safety
	///
	/// The caller must ensure the resource protected by the spinlock is not in use before calling
	/// this function. Otherwise, data races might happen.
	#[inline(always)]
	pub unsafe fn unlock(&mut self) {
		self.locked.store(false, Ordering::Release);
	}
}
