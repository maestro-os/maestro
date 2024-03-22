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

//! In the malloc allocator, a chunk of memory is a subdivision of a [`Block`].
//!
//! It represents a portion of memory that might be available for allocation, or
//! might be already allocated.
//!
//! Chunks of the same blocks are linked to each others by a double linked list, which allows to
//! split and merge nearby chunks when necessary.
//!
//! If a chunk is not allocated, it is stored in a free list, stored by size.

use super::block::Block;
use core::{
	cmp::{max, min},
	mem::size_of,
	num::NonZeroUsize,
	ptr,
	ptr::{addr_of_mut, NonNull},
};
use utils::errno::AllocResult;

/// The magic number for every chunks
#[cfg(config_debug_malloc_magic)]
const CHUNK_MAGIC: u32 = 0xdeadbeef;
/// Chunk flag indicating that the chunk is being used
const CHUNK_FLAG_USED: u8 = 0b1;
/// The minimum amount of bytes required to create a free chunk.
const FREE_CHUNK_MIN: usize = 8;
/// The required alignment for pointers returned by allocator.
pub const ALIGNMENT: usize = 8;
/// The size of the smallest free list bin.
const FREE_LIST_SMALLEST_SIZE: usize = FREE_CHUNK_MIN;
/// The number of free list bins.
const FREE_LIST_BINS: usize = 8;

/// A chunk of allocated or free memory, stored in linked lists.
#[repr(align(8))]
pub struct Chunk {
	/// The magic number to check integrity of the chunk.
	#[cfg(config_debug_malloc_magic)]
	magic: u32,

	/// The previous chunk in the block.
	prev: Option<NonNull<Self>>,
	/// The next chunk in the block.
	next: Option<NonNull<Self>>,

	/// Whether the chunk is in use
	pub used: bool,
	/// The size of the chunk's memory in bytes
	size: usize,
}

impl Chunk {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			#[cfg(config_debug_malloc_magic)]
			magic: CHUNK_MAGIC,

			prev: None,
			next: None,

			used: false,
			size: 0,
		}
	}

	/// Returns the chunk corresponding to the given data pointer.
	pub unsafe fn from_ptr(ptr: *mut u8) -> &'static mut Self {
		&mut *(((ptr as usize) - size_of::<Self>()) as *mut Self)
	}

	/// Returns the previous chunk.
	#[inline]
	pub fn get_prev(&self) -> Option<&'static mut Self> {
		self.prev.map(|mut n| unsafe { n.as_mut() })
	}

	/// Returns the next chunk.
	#[inline]
	pub fn get_next(&self) -> Option<&'static mut Self> {
		self.next.map(|mut n| unsafe { n.as_mut() })
	}

	/// Tells whether the chunk is disconnected from a chunk list.
	#[inline]
	pub fn is_single(&self) -> bool {
		self.prev.is_none() && self.next.is_none()
	}

	/// Inserts the current chunk after the given one.
	pub fn insert_after(&mut self, chunk: &mut Self) {
		debug_assert!((chunk as *const _ as usize) < (self as *const _ as usize));

		self.prev = NonNull::new(chunk);
		self.next = chunk.next;
		if let Some(next) = self.get_next() {
			next.prev = NonNull::new(self);
		}
		chunk.next = NonNull::new(self);
	}

	/// Unlinks the current chunks from the list.
	pub fn unlink(&mut self) {
		if let Some(prev) = self.get_prev() {
			prev.next = self.next;
		}
		if let Some(next) = self.get_next() {
			next.prev = self.prev;
		}
		self.prev = None;
		self.next = None;
	}

	/// Returns an immutable pointer to the chunks' data.
	#[inline]
	pub fn get_ptr(&self) -> *const u8 {
		unsafe { (self as *const Self as *const u8).add(size_of::<Self>()) }
	}

	/// Returns a mutable pointer to the chunks' data.
	#[inline]
	pub fn get_ptr_mut(&mut self) -> *mut u8 {
		unsafe { (self as *mut Self as *mut u8).add(size_of::<Self>()) }
	}

	/// Returns the size of the chunk.
	#[inline]
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Checks that the chunk is correct. This function uses assertions and thus
	/// is useful only in debug mode.
	#[cfg(config_debug_malloc_check)]
	pub fn check(&self) {
		#[cfg(config_debug_malloc_magic)]
		debug_assert_eq!(self.magic, CHUNK_MAGIC);

		debug_assert!(self as *const _ as usize >= crate::memory::PROCESS_END as usize);
		debug_assert!(self.get_size() >= get_min_chunk_size());

		if let Some(prev) = self.get_prev() {
			debug_assert!(prev as *const _ as usize >= crate::memory::PROCESS_END as usize);

			#[cfg(config_debug_malloc_magic)]
			debug_assert_eq!(prev.magic, CHUNK_MAGIC);

			debug_assert!((prev as *const Self as usize) < (self as *const Self as usize));
			debug_assert!(prev.get_size() >= get_min_chunk_size());
			debug_assert!(
				(prev.get_ptr() as usize) + prev.get_size() <= (self as *const Self as usize)
			);
		}

		if let Some(next) = self.get_next() {
			debug_assert!(next as *const _ as usize >= crate::memory::PROCESS_END as usize);

			#[cfg(config_debug_malloc_magic)]
			debug_assert_eq!(next.magic, CHUNK_MAGIC);

			debug_assert!((self as *const Self as usize) < (next as *const Self as usize));
			debug_assert!(next.get_size() >= get_min_chunk_size());
			debug_assert!(
				(self.get_ptr() as usize) + self.get_size() <= (next as *const Self as usize)
			);
		}

		debug_assert!(self.get_ptr().is_aligned_to(ALIGNMENT));
	}

	/// Returns a mutable reference for the given chunk as a free chunk.
	///
	/// If the chunk is used, the function returns `None`.
	#[inline]
	pub fn as_free_chunk(&mut self) -> Option<&mut FreeChunk> {
		if !self.used {
			let c = unsafe { &mut *(self as *mut Self as *mut FreeChunk) };
			Some(c)
		} else {
			None
		}
	}

	/// Returns the pointer to the next chunk for splitting the current chunk
	/// with given size `size`.
	///
	/// If the chunk cannot be split, the function returns `None`.
	fn get_split_next_chunk(&mut self, size: usize) -> Option<&'static mut FreeChunk> {
		#[cfg(config_debug_malloc_check)]
		self.check();
		let min_data_size = get_min_chunk_size();
		let size = max(size, min_data_size);
		let next_ptr = unsafe { utils::align(self.get_ptr().add(size), ALIGNMENT) };
		let new_size = (next_ptr as usize) - (self.get_ptr() as usize);
		debug_assert!(new_size >= size);
		if new_size + size_of::<Chunk>() + min_data_size <= self.size {
			Some(unsafe { &mut *(next_ptr as *mut FreeChunk) })
		} else {
			None
		}
	}

	/// Splits the chunk with the given size `size` if necessary.
	///
	/// The function might create a new free chunk next to the current, in which case, it is
	/// returned.
	///
	/// The current chunk is removed from the free list if present.
	///
	/// The current chunk is *not* inserted into the free list, but the next one is inserted.
	pub fn split(&mut self, size: usize) -> Option<&'static mut FreeChunk> {
		if let Some(free_chunk) = self.as_free_chunk() {
			free_chunk.free_list_remove();
		}
		// Create next chunk
		let next = self.get_split_next_chunk(size)?;
		let new_size = (next as *mut _ as usize) - (self.get_ptr() as usize);
		let next_size = self.size - new_size - size_of::<Chunk>();
		unsafe {
			ptr::write_volatile(next, FreeChunk::new(next_size));
		}
		#[cfg(config_debug_malloc_check)]
		next.check();
		next.free_list_insert();
		next.chunk.insert_after(self);
		// Update current chunk
		self.size = new_size;
		#[cfg(config_debug_malloc_check)]
		self.check();
		Some(next)
	}

	/// Tries to coalesce the chunk it with adjacent chunks if they are free.
	///
	/// The current chunk is removed from the free list if present.
	///
	/// The function returns the resulting chunk.
	///
	/// The resulting chunk is *not* inserted into the free list.
	pub fn coalesce(&mut self) -> &mut Chunk {
		if let Some(free_chunk) = self.as_free_chunk() {
			free_chunk.free_list_remove();
		}
		if let Some(next) = self.get_next().and_then(Chunk::as_free_chunk) {
			next.free_list_remove();
			next.chunk.unlink();
			// Update size and free list bucket
			self.size += size_of::<Chunk>() + next.chunk.size;
		}
		if let Some(prev) = self.get_prev() {
			if !prev.used {
				// Termination is guaranteed because two free chunks are always coalesced
				// immediately
				return prev.coalesce();
			}
		}
		#[cfg(config_debug_malloc_check)]
		self.check();
		self
	}

	/// Tries to grow the given chunk of `delta` more bytes.
	///
	/// If not possible, the function returns `false`.
	///
	/// The function might alter the free list to get the space needed.
	pub fn grow(&mut self, delta: usize) -> bool {
		debug_assert!(self.used);
		debug_assert_ne!(delta, 0);

		let new_size = self.size + delta;

		// Check A
		let Some(next) = self.get_next() else {
			return false;
		};

		// Check B
		let available_size = size_of::<Chunk>() + next.size;
		if available_size < delta {
			return false;
		}

		// Check C
		let Some(next_free) = next.as_free_chunk() else {
			return false;
		};

		// Action C
		next_free.free_list_remove();

		// Action B
		self.size += available_size;

		// Action A
		next.unlink();

		// Split the current chunk to get the right size
		self.split(new_size);

		#[cfg(config_debug_malloc_check)]
		self.check();
		#[cfg(config_debug_malloc_check)]
		check_free_lists();

		true
	}

	/// Tries to shrink the given chunk of `delta` less bytes.
	///
	/// If the chunk is not used, the function does nothing.
	///
	/// The function might alter the free list to relinquish the space.
	pub fn shrink(&mut self, delta: usize) {
		debug_assert_ne!(delta, 0);
		debug_assert!(delta < self.size);

		let new_size = max(self.size - delta, get_min_chunk_size());
		if let Some(next) = self.split(new_size) {
			next.chunk.coalesce();
		}

		#[cfg(config_debug_malloc_check)]
		self.check();
		#[cfg(config_debug_malloc_check)]
		check_free_lists();
	}
}

/// A free chunk, wrapping the Chunk structure.
///
/// The representation of the structure doesn't allow fields reordering.
///
/// This is because the linked list for the list of free chunks needs to be located after the
/// chunks header, in order to use the chunk's body to store it.
#[repr(C, align(8))]
pub struct FreeChunk {
	/// The chunk
	pub chunk: Chunk,

	/// The previous chunk in the free list.
	pub prev: Option<NonNull<Self>>,
	/// The next chunk in the free list.
	pub next: Option<NonNull<Self>>,
}

impl FreeChunk {
	/// Creates a new free chunk with the given size `size` in bytes and returns it.
	///
	/// The chunk is **not** inserted into the free list.
	pub fn new(size: usize) -> Self {
		Self {
			prev: None,
			next: None,

			chunk: Chunk {
				#[cfg(config_debug_malloc_magic)]
				magic: CHUNK_MAGIC,

				prev: None,
				next: None,

				used: false,
				size,
			},
		}
	}

	/// Checks that the chunk is correct.
	///
	/// This function uses assertions and thus is useful only in debug mode.
	#[cfg(config_debug_malloc_check)]
	pub fn check(&self) {
		assert!(!self.chunk.used);
		self.chunk.check();
	}

	/// Inserts the chunk into the appropriate free list.
	pub fn free_list_insert(&mut self) {
		#[cfg(config_debug_malloc_check)]
		debug_assert!(!self.chunk.used);
		debug_assert!(self.prev.is_none());
		debug_assert!(self.next.is_none());

		// Cannot fail since `get_free_list` cannot return `None` when `splittable` is `false`
		let free_list = get_free_list(self.chunk.size, false).unwrap();
		debug_assert_ne!(*free_list, NonNull::new(self));

		self.next = *free_list;
		if let Some(mut next) = self.next {
			unsafe { next.as_mut() }.prev = NonNull::new(self);
		}
		*free_list = NonNull::new(self);

		#[cfg(config_debug_malloc_check)]
		check_free_lists();
	}

	/// Removes the chunk from its free list.
	pub fn free_list_remove(&mut self) {
		// Cannot panic since `get_free_list` cannot return `None` when `splittable` is `false`
		let free_list = get_free_list(self.chunk.size, false).unwrap();

		let is_front = free_list.map(|c| c.as_ptr() == self).unwrap_or(false);
		if is_front {
			*free_list = self.next;
		}

		if let Some(mut prev) = self.prev {
			unsafe { prev.as_mut() }.next = self.next;
		}
		if let Some(mut next) = self.next {
			unsafe { next.as_mut() }.prev = self.prev;
		}
		self.prev = None;
		self.next = None;

		#[cfg(config_debug_malloc_check)]
		check_free_lists();
	}
}

/// List storing free lists for each free chunk. The chunks are sorted by size.
static mut FREE_LISTS: [Option<NonNull<FreeChunk>>; FREE_LIST_BINS] = [None; FREE_LIST_BINS];

/// Returns the minimum data size for a chunk.
const fn get_min_chunk_size() -> usize {
	let len = size_of::<FreeChunk>() - size_of::<Chunk>();
	// Required because `max` is not `const`
	if len > FREE_CHUNK_MIN {
		len
	} else {
		FREE_CHUNK_MIN
	}
}

/// Checks the chunks inside each free lists.
#[cfg(config_debug_malloc_check)]
fn check_free_lists() {
	// Safe because the usage of the malloc API is secured by a Mutex
	// FIXME: this is dirty
	let free_lists = unsafe { &mut *addr_of_mut!(FREE_LISTS) };
	for free_list in free_lists {
		let mut node = *free_list;
		while let Some(mut n) = node {
			let n = unsafe { n.as_mut() };
			n.check();
			node = n.next;
		}
	}
}

/// Returns the free list for the given size `size`.
///
/// If `splittable` is set, the function may return a free list that contain chunks greater than
/// the required size so that it can be split.
fn get_free_list(
	size: usize,
	splittable: bool,
) -> Option<&'static mut Option<NonNull<FreeChunk>>> {
	#[cfg(config_debug_malloc_check)]
	check_free_lists();
	// Safe because the usage of the malloc API is secured by a Mutex
	// FIXME: this is dirty
	let free_lists = unsafe { &mut *addr_of_mut!(FREE_LISTS) };
	let i = min(
		(size / FREE_LIST_SMALLEST_SIZE)
			.checked_ilog2()
			.unwrap_or(0) as usize,
		FREE_LIST_BINS - 1,
	);
	if splittable {
		free_lists[(i + 1)..].iter_mut().find(|l| l.is_some())
	} else {
		Some(&mut free_lists[i])
	}
}

/// Returns a reference to a free chunk suitable for an allocation of given size
/// `size`.
///
/// On success, the return value MUST be used or might result in a
/// memory leak.
pub fn get_available_chunk(size: NonZeroUsize) -> AllocResult<&'static mut FreeChunk> {
	let free_list = get_free_list(size.get(), true);
	let free_chunk = if let Some(f) = free_list {
		unsafe { f.unwrap().as_mut() }
	} else {
		let block = Block::new(size)?;
		block.first_chunk.as_free_chunk().unwrap()
	};
	#[cfg(config_debug_malloc_check)]
	free_chunk.check();
	debug_assert!(free_chunk.chunk.size >= size.get());
	debug_assert!(!free_chunk.chunk.used);
	Ok(free_chunk)
}
