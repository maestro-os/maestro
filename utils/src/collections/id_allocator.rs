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

use crate::{collections::bitfield::Bitfield, errno::AllocResult};
use core::alloc::AllocError;

/// Structure representing an identifier allocator.
pub struct IDAllocator {
	/// The bitfield keeping track of used identifiers.
	used: Bitfield,
}

impl IDAllocator {
	/// Creates a new instance.
	///
	/// `max` is the maximum ID.
	pub fn new(max: u32) -> AllocResult<Self> {
		Ok(Self {
			used: Bitfield::new((max + 1) as _)?,
		})
	}

	/// Sets the id `id` as used.
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
	#[must_use = "not freeing a PID shall cause a leak"]
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
