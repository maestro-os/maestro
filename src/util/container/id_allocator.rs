//! This module implements an identifier allocator, allowing to allocate and free indexes in a
//! range between [0; max], where `max` is given.

use crate::errno::Errno;
use crate::errno;
use crate::util::container::bitfield::Bitfield;

/// Structure representing an identifier allocator.
pub struct IDAllocator {
	/// The bitfield keeping track of used identifiers.
	used: Bitfield,
}

impl IDAllocator {
	/// Creates a new instance.
	/// `max` is the maximum id.
	pub fn new(max: u32) -> Result<Self, Errno> {
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
	/// If `id` is not None, the function shall allocate the given id.
	/// If the allocation fails, the function returns an Err.
	pub fn alloc(&mut self, id: Option<u32>) -> Result<u32, Errno> {
		if let Some(i) = id {
			if !self.used.is_set(i as _) {
				self.used.set(i as _);
				Ok(i)
			} else {
				Err(errno::ENOMEM)
			}
		} else if let Some(i) = self.used.find_clear() {
			self.used.set(i);
			Ok(i as _)
		} else {
			Err(errno::ENOMEM)
		}
	}

	/// Frees the given identifier `id`.
	pub fn free(&mut self, id: u32) {
		if id <= self.used.len() as _ {
			self.used.clear(id as _);
		}
	}
}
