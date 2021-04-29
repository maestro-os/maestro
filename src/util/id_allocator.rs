/// This module implements an identifier allocator, allowing to allocate and free indexes in a
/// given range.

use crate::errno::Errno;
use crate::errno;
use crate::util::container::vec::Vec;

// TODO Unit tests
/// Looks for a hole in the vector `v`. If no hole is found, the function returns None.
/// The vector must be sorted in growing order. If not, the behaviour is undefined.
fn find_hole(v: &Vec<u32>) -> Option<usize> {
	if !v.is_empty() {
		let mut i = v.len() / 2;
		let mut step_size = i / 2;

		while i > 0 {
			if v[i] > i as _ {
				i -= step_size;
			} else {
				i += step_size;
			}

			step_size /= 2;
		}

		if i < v.len() && v[i] != i as _ {
			Some(i)
		} else {
			None
		}
	} else {
		None
	}
}

/// Structure representing an identifier allocator.
pub struct IDAllocator {
	/// The maximum identifier.
	max: Option<u32>,
	/// The list of used indexes.
	used: Vec<u32>,
}

impl IDAllocator {
	/// Creates a new instance. If specified, the identifiers to be allocated are less than `max`.
	pub const fn new(max: Option<u32>) -> Self {
		Self {
			max: max,
			used: Vec::<u32>::new(),
		}
	}

	/// Allocates an identifier.
	/// If `id` is not None, the function shall allocate the specific given id.
	/// If the allocation fails, the function returns an Err.
	pub fn alloc(&mut self, id: Option<u32>) -> Result<u32, Errno> {
		if self.used.is_empty() {
			let id = if let Some(id) = id {
				id
			} else {
				0
			};

			self.used.push(id)?;
			Ok(id)
		} else {
			let (index, id) = if let Some(id) = id {
				if let Err(index) = self.used.binary_search(&id) {
					(index, self.used[index] - 1)
				} else {
					return Err(errno::ENOMEM);
				}
			} else {
				if let Some(index) = find_hole(&self.used) {
					(index, self.used[index] - 1)
				} else {
					return Err(errno::ENOMEM);
				}
			};

			if let Some(max) = self.max {
				if id > max {
					return Err(errno::ENOMEM);
				}
			}

			self.used.insert(index, id)?;
			Ok(id)
		}
	}

	/// Frees the given identifier `id`.
	pub fn free(&mut self, id: u32) {
		if let Ok(index) = self.used.binary_search(&id) {
			self.used.remove(index);
		} else {
			crate::kernel_panic!("Freeing identifier that isn't allocated!", 0);
		}
	}
}

// TODO Unit tests
