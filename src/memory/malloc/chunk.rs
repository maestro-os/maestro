//! In the malloc allocator, a chunk of memory is a subdivision of a `Block`.
//!
//! It represents a portion of memory that might be available for allocation, or
//! might be already allocated.
//!
//! Chunks of the same blocks are linked to each others by a double linked list, which allows to
//! split and merge nearby chunks when necessary.
//!
//! If a chunk is not allocated, it is stored in a free list, stored by size.

use super::block::Block;
use crate::errno::Errno;
use crate::util;
use core::cmp::{max, min};
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use core::ptr::NonNull;

/// The magic number for every chunks
#[cfg(config_debug_malloc_magic)]
const CHUNK_MAGIC: u32 = 0xdeadbeef;

/// Chunk flag indicating that the chunk is being used
const CHUNK_FLAG_USED: u8 = 0b1;

/// The minimum amount of bytes required to create a free chunk.
const FREE_CHUNK_MIN: usize = 8;

/// The required alignement for pointers returned by allocator.
pub const ALIGNEMENT: usize = 8;

/// The size of the smallest free list bin.
const FREE_LIST_SMALLEST_SIZE: usize = FREE_CHUNK_MIN;

/// The number of free list bins.
const FREE_LIST_BINS: usize = 8;

/// A chunk of allocated or free memory stored in linked lists.
#[repr(align(8))]
pub struct Chunk {
	/// The magic number to check integrity of the chunk.
	#[cfg(config_debug_malloc_magic)]
	magic: u32,

	/// The previous chunk in the block.
	prev: Option<NonNull<Self>>,
	/// The next chunk in the block.
	next: Option<NonNull<Self>>,

	/// The chunk's flags
	flags: u8,
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

			flags: 0,
			size: 0,
		}
	}

	/// Returns the chunk corresponding to the given data pointer.
	pub unsafe fn from_ptr(ptr: *mut c_void) -> &'static mut Self {
		&mut *(((ptr as usize) - size_of::<Self>()) as *mut Self)
	}

	/// Returns the previous chunk.
	pub fn get_prev<'s>(&self) -> Option<&'static mut Self> {
		self.prev.map(|mut n| unsafe { n.as_mut() })
	}

	/// Returns the next chunk.
	pub fn get_next(&self) -> Option<&'static mut Self> {
		self.next.map(|mut n| unsafe { n.as_mut() })
	}

	/// Tells whether the chunk is disconnected from a chunk list.
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

	/// Tells the whether the chunk is free.
	#[inline]
	pub fn is_used(&self) -> bool {
		(self.flags & CHUNK_FLAG_USED) != 0
	}

	/// Sets the chunk used or free.
	#[inline]
	pub fn set_used(&mut self, used: bool) {
		#[cfg(config_debug_malloc_check)]
		self.check();

		if used {
			self.flags |= CHUNK_FLAG_USED;
		} else {
			self.flags &= !CHUNK_FLAG_USED;
		}

		#[cfg(config_debug_malloc_check)]
		self.check();
	}

	/// Returns an immutable pointer to the chunks' data.
	#[inline]
	pub fn get_ptr(&self) -> *const c_void {
		unsafe { (self as *const Self as *const c_void).add(size_of::<Self>()) }
	}

	/// Returns a mutable pointer to the chunks' data.
	#[inline]
	pub fn get_ptr_mut(&mut self) -> *mut c_void {
		unsafe { (self as *mut Self as *mut c_void).add(size_of::<Self>()) }
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

		debug_assert!(util::is_aligned(self.get_ptr(), ALIGNEMENT));
	}

	/// Returns a mutable reference for the given chunk as a free chunk.
	///
	/// If the chunk is used, the function returns `None`.
	#[inline]
	pub fn as_free_chunk(&mut self) -> Option<&mut FreeChunk> {
		if !self.is_used() {
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
	fn get_split_next_chunk(&mut self, size: usize) -> Option<*mut FreeChunk> {
		let min_data_size = get_min_chunk_size();
		let size = max(size, min_data_size);

		let next_ptr = util::align(unsafe { self.get_ptr().add(size) }, ALIGNEMENT);

		let new_size = (next_ptr as usize) - (self.get_ptr() as usize);
		debug_assert!(new_size >= size);

		if new_size + size_of::<Chunk>() + min_data_size <= self.get_size() {
			Some(next_ptr as *mut FreeChunk)
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
		#[cfg(config_debug_malloc_check)]
		self.check();
		debug_assert!(self.get_size() >= size);

		if let Some(free_chunk) = self.as_free_chunk() {
			free_chunk.free_list_remove();
		}

		let res = if let Some(next_ptr) = self.get_split_next_chunk(size) {
			let new_size = (next_ptr as usize) - (self.get_ptr() as usize);
			let next_size = self.size - new_size - size_of::<Chunk>();

			let next = unsafe {
				ptr::write_volatile(next_ptr, FreeChunk::new(next_size));
				&mut *next_ptr
			};

			#[cfg(config_debug_malloc_check)]
			next.check();

			next.free_list_insert();
			next.chunk.insert_after(self);

			self.size = new_size;

			Some(next)
		} else {
			None
		};

		#[cfg(config_debug_malloc_check)]
		self.check();
		debug_assert!(self.get_size() >= size);

		res
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

		if let Some(next) = self.get_next() {
			if let Some(next_free) = next.as_free_chunk() {
				next_free.free_list_remove();
				drop(next_free);
				next.unlink();

				// Update size and free list bucket
				self.size += size_of::<Chunk>() + next.size;
			}
		}

		if let Some(prev) = self.get_prev() {
			if !prev.is_used() {
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
		debug_assert!(self.is_used());
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
		drop(next_free);

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
		debug_assert!(delta < self.get_size());

		let new_size = max(self.get_size() - delta, get_min_chunk_size());
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

				flags: 0,
				size,
			},
		}
	}

	/// Checks that the chunk is correct.
	///
	/// This function uses assertions and thus is useful only in debug mode.
	#[cfg(config_debug_malloc_check)]
	pub fn check(&self) {
		assert!(!self.chunk.is_used());
		self.chunk.check();
	}

	/// Inserts the chunk into the appropriate free list.
	pub fn free_list_insert(&mut self) {
		#[cfg(config_debug_malloc_check)]
		debug_assert!(!self.chunk.is_used());
		debug_assert!(self.prev.is_none());
		debug_assert!(self.next.is_none());

		// Cannot panic since `get_free_list` cannot return `None` when `splittable` is `false`
		let free_list = get_free_list(self.chunk.size, false).unwrap();
		debug_assert!(*free_list != NonNull::new(self));

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

/// List storing free lists for each free chunk. The chunks are storted by size.
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

/// Checks the chunks inside of each free lists.
#[cfg(config_debug_malloc_check)]
fn check_free_lists() {
	// Safe because the usage of the malloc API is secured by a Mutex
	let free_lists = unsafe { &mut FREE_LISTS };

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

	let mut i = size / FREE_LIST_SMALLEST_SIZE;
	if i > 0 {
		i = i.ilog2() as usize;
	}
	i = min(i, FREE_LIST_BINS - 1);

	// Safe because the usage of the malloc API is secured by a Mutex
	let free_lists = unsafe { &mut FREE_LISTS };

	if splittable {
		i += 1;

		while i < FREE_LIST_BINS && free_lists[i].is_none() {
			i += 1;
		}

		if i >= FREE_LIST_BINS {
			return None;
		}
	}

	Some(&mut free_lists[i])
}

/// Returns a reference to a free chunk suitable for an allocation of given size
/// `size`.
///
/// On success, the return value MUST be used or might result in a
/// memory leak.
pub fn get_available_chunk(size: usize) -> Result<&'static mut FreeChunk, Errno> {
	let free_list = get_free_list(size, true);
	let free_chunk = if let Some(f) = free_list {
		unsafe { f.unwrap().as_mut() }
	} else {
		let block = Block::new(size)?;
		unsafe { &mut *(&mut block.first_chunk as *mut _ as *mut FreeChunk) }
	};

	#[cfg(config_debug_malloc_check)]
	free_chunk.check();
	debug_assert!(free_chunk.chunk.size >= size);
	debug_assert!(!free_chunk.chunk.is_used());

	Ok(free_chunk)
}
