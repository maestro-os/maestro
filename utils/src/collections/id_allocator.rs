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

//! This module implements an identifier allocator, allowing to allocate and
//! free indexes in range `0..=max`, where `max` is given.

use crate::{
	collections::{bitfield::Bitfield, vec::Vec},
	errno::AllocResult,
};
use core::alloc::AllocError;

/// An identifier allocator, based upon a bitfield
pub struct IDAllocator<C: AsRef<[u8]> + AsMut<[u8]> = Vec<u8>> {
	/// The bitfield keeping track of used identifiers.
	used: Bitfield<C>,
}

impl IDAllocator<Vec<u8>> {
	/// Creates a new allocated, allocated on the heap
	///
	/// `max` is the maximum ID
	pub fn new_allocated(max: u32) -> AllocResult<Self> {
		Ok(Self {
			used: Bitfield::new_allocated((max + 1) as _)?,
		})
	}
}

impl<const N: usize> IDAllocator<[u8; N]> {
	/// Creates a new allocated, stored in place
	///
	/// The length is `N * 8`
	pub const fn new_inplace() -> Self {
		Self {
			used: Bitfield::new_inplace(),
		}
	}
}

impl<C: AsRef<[u8]> + AsMut<[u8]>> IDAllocator<C> {
	/// Tells whether `id` is marked as used.
	///
	/// If out of bounds, the function returns `true`.
	pub fn is_used(&self, id: u32) -> bool {
		if id <= self.used.len() as _ {
			self.used.is_set(id as _)
		} else {
			true
		}
	}

	/// Sets `id` as used.
	pub fn set_used(&mut self, id: u32) {
		if id <= self.used.len() as _ {
			self.used.set(id as _);
		}
	}

	/// Allocates an identifier.
	///
	/// If `id` is not `None`, the function shall allocate the given id.
	///
	/// If the allocation fails, the function returns `None`.
	pub fn alloc(&mut self, id: Option<u32>) -> AllocResult<u32> {
		if let Some(i) = id {
			if !self.used.is_set(i as _) {
				self.used.set(i as _);
				Ok(i)
			} else {
				Err(AllocError)
			}
		} else if let Some(i) = self.used.find_clear() {
			self.used.set(i);
			Ok(i as _)
		} else {
			Err(AllocError)
		}
	}

	/// Frees the given identifier `id`.
	pub fn free(&mut self, id: u32) {
		if id <= self.used.len() as _ {
			self.used.clear(id as _);
		}
	}
}
