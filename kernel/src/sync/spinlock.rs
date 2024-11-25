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

//! Spinlock implementation.

use core::{
	hint,
	sync::{atomic, atomic::AtomicBool},
};

/// Locking primitive spinning until the resource can be acquired.
///
/// It works by storing a value telling whether a thread is already in that piece of code.
///
/// To avoid race conditions, the implementation uses an atomic exchange instruction. If a threads
/// tries to acquire the lock while already in use, the thread shall wait in a loop (spin) until
/// the lock is released.
pub struct Spinlock(AtomicBool);

impl Spinlock {
	/// Creates a new spinlock.
	#[allow(clippy::new_without_default)]
	pub const fn new() -> Self {
		Self(AtomicBool::new(false))
	}

	/// Locks the spinlock.
	#[inline(always)]
	pub fn lock(&mut self) {
		while self.0.swap(true, atomic::Ordering::Acquire) {
			hint::spin_loop();
		}
	}

	/// Unlocks the spinlock.
	#[inline(always)]
	pub fn unlock(&mut self) {
		self.0.store(false, atomic::Ordering::Release);
	}
}
