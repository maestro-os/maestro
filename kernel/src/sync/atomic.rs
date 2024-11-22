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

//! Implementation of [`AtomicU64`].

use core::{fmt, fmt::Formatter, sync::atomic};

/// Fulfills the role of `AtomicU64`, while being available on 32 bits platforms.
#[cfg(target_has_atomic = "64")]
#[derive(Default)]
pub struct AtomicU64(core::sync::atomic::AtomicU64);

/// Fulfills the role of `AtomicU64`, while being available on 32 bits platforms.
#[cfg(not(target_has_atomic = "64"))]
#[derive(Default)]
pub struct AtomicU64(super::IntMutex<u64>);

impl AtomicU64 {
	/// Creates a new instance with the given value.
	pub const fn new(val: u64) -> Self {
		#[cfg(target_has_atomic = "64")]
		{
			Self(core::sync::atomic::AtomicU64::new(val))
		}
		#[cfg(not(target_has_atomic = "64"))]
		{
			Self(super::IntMutex::new(val))
		}
	}

	/// Loads a value from the atomic integer.
	#[allow(unused_variables)]
	pub fn load(&self, order: atomic::Ordering) -> u64 {
		#[cfg(target_has_atomic = "64")]
		{
			self.0.load(order)
		}
		#[cfg(not(target_has_atomic = "64"))]
		{
			*self.0.lock()
		}
	}

	/// Stores a value into the atomic integer.
	#[allow(unused_variables)]
	pub fn store(&self, val: u64, order: atomic::Ordering) {
		#[cfg(target_has_atomic = "64")]
		{
			self.0.store(val, order)
		}
		#[cfg(not(target_has_atomic = "64"))]
		{
			*self.0.lock() = val;
		}
	}

	/// Adds to the current value, returning the previous value.
	#[allow(unused_variables)]
	pub fn fetch_add(&self, val: u64, order: atomic::Ordering) -> u64 {
		#[cfg(target_has_atomic = "64")]
		{
			self.0.fetch_add(val, order)
		}
		#[cfg(not(target_has_atomic = "64"))]
		{
			let mut guard = self.0.lock();
			let prev = *guard;
			*guard = guard.wrapping_add(val);
			prev
		}
	}
}

impl fmt::Debug for AtomicU64 {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.0, f)
	}
}
