//! In the malloc allocator, a chunk of memory is a subdivision of a Block. It represents a portion
//! of memory that might be available for allocation, or might be already allocated.
//! Chunks of the same blocks are linked to each others by a double linked list, which allows to
//! split and merge nearby chunks when necessary.
//! If a chunk is not allocated, it is stored in a free list, stored by size.

use core::cmp::{min, max};
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::mem::size_of;
use core::ptr;
use crate::errno::Errno;
use crate::list_new;
use crate::util::list::List;
use crate::util::list::ListNode;
use crate::util::math;
use crate::util;
use super::block::Block;

/// Type representing chunks' flags.
type ChunkFlags = u8;

/// The magic number for every chunks
#[cfg(config_debug_malloc_magic)]
const CHUNK_MAGIC: u32 = 0xdeadbeef;

/// Chunk flag indicating that the chunk is being used
const CHUNK_FLAG_USED: ChunkFlags = 0b1;

/// The minimum amount of bytes required to create a free chunk.
const FREE_CHUNK_MIN: usize = 8;

/// The required alignement for pointers returned by allocator.
pub const ALIGNEMENT: usize = 8;

/// The size of the smallest free list bin.
const FREE_LIST_SMALLEST_SIZE: usize = FREE_CHUNK_MIN;

/// The number of free list bins.
const FREE_LIST_BINS: usize = 8;

/// Type representing a free list entry into the free lists list.
type FreeList = List<FreeChunk>;

/// List storing free lists for each free chunk. The chunks are storted by size.
static mut FREE_LISTS: MaybeUninit<[List<FreeChunk>; FREE_LIST_BINS]> = MaybeUninit::uninit();

/// A chunk of allocated or free memory stored in linked lists.
#[repr(C, align(8))]
pub struct Chunk {
	/// The magic number to check integrity of the chunk.
	#[cfg(config_debug_malloc_magic)]
	magic: u32,
	/// The linked list storing the chunks
	pub list: ListNode,
	/// The chunk's flags
	flags: u8,
	/// The size of the chunk's memory in bytes
	size: usize,
}

/// A free chunk, wrapping the Chunk structure.
#[repr(C, align(8))]
pub struct FreeChunk {
	/// The chunk
	chunk: Chunk,
	/// The linked list for the free list
	pub free_list: ListNode,
}

impl Chunk {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			#[cfg(config_debug_malloc_magic)]
			magic: CHUNK_MAGIC,
			list: ListNode::new_single(),
			flags: 0,
			size: 0,
		}
	}

	/// Returns the chunk corresponding to the given data pointer.
	pub unsafe fn from_ptr(ptr: *mut c_void) -> &'static mut Self {
		&mut *(((ptr as usize) - size_of::<Self>()) as *mut Self)
	}

	/// Tells the whether the chunk is free.
	pub fn is_used(&self) -> bool {
		(self.flags & CHUNK_FLAG_USED) != 0
	}

	/// Sets the chunk used or free.
	pub fn set_used(&mut self, used: bool) {
		#[cfg(config_debug_debug)]
		self.check();

		if used {
			self.flags |= CHUNK_FLAG_USED;
		} else {
			self.flags &= !CHUNK_FLAG_USED;
		}

		#[cfg(config_debug_debug)]
		self.check();
	}

	/// Returns a pointer to the chunks' data.
	pub fn get_ptr(&mut self) -> *mut c_void {
		unsafe {
			(self as *mut Self as *mut c_void).add(size_of::<Self>())
		}
	}

	/// Returns a const pointer to the chunks' data.
	pub fn get_const_ptr(&self) -> *const c_void {
		unsafe {
			(self as *const Self as *const c_void).add(size_of::<Self>())
		}
	}

	/// Returns the size of the chunk.
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Checks that the chunk is correct. This function uses assertions and thus is useful only in
	/// debug mode.
	#[cfg(config_debug_debug)]
	pub fn check(&self) {
		#[cfg(config_debug_malloc_magic)]
		debug_assert_eq!(self.magic, CHUNK_MAGIC);

		debug_assert!(self as *const _ as usize >= crate::memory::PROCESS_END as usize);
		debug_assert!(self.get_size() >= get_min_chunk_size());

		if let Some(prev) = self.list.get_prev() {
			let p = prev.get::<Chunk>(crate::offset_of!(Chunk, list));
			debug_assert!(p as *const _ as usize >= crate::memory::PROCESS_END as usize);

			#[cfg(config_debug_malloc_magic)]
			debug_assert_eq!(p.magic, CHUNK_MAGIC);

			debug_assert!(p.get_size() >= get_min_chunk_size());
			debug_assert!((p.get_const_ptr() as usize) + p.get_size()
				<= (self as *const Self as usize));
		}

		if let Some(next) = self.list.get_next() {
			let n = next.get::<Chunk>(crate::offset_of!(Chunk, list));
			debug_assert!(n as *const _ as usize >= crate::memory::PROCESS_END as usize);

			#[cfg(config_debug_malloc_magic)]
			debug_assert_eq!(n.magic, CHUNK_MAGIC);

			debug_assert!(n.get_size() >= get_min_chunk_size());
			debug_assert!((self.get_const_ptr() as usize) + self.get_size()
				<= (n as *const Self as usize));
		}

		debug_assert!(util::is_aligned(self.get_const_ptr(), ALIGNEMENT));
	}

	/// Returns a mutable reference for the given chunk as a free chunk. The result is undefined if
	/// the chunk is used.
	pub fn as_free_chunk(&mut self) -> &mut FreeChunk {
		debug_assert!(!self.is_used());

		unsafe {
			&mut *(self as *mut Self as *mut FreeChunk)
		}
	}

	/// Returns the reference to the next chunk for splitting the current chunk with given size
	/// `size`. If the chunk cannot be split, the function returns None.
	fn get_split_next_chunk(&mut self, size: usize) -> Option::<*mut FreeChunk> {
		let min_data_size = get_min_chunk_size();
		let next_ptr = util::align(unsafe {
			self.get_ptr().add(max(size, min_data_size))
		}, ALIGNEMENT);

		let curr_new_size = (next_ptr as usize) - (self.get_ptr() as usize);
		debug_assert!(curr_new_size >= size);

		if curr_new_size + size_of::<Chunk>() + min_data_size <= self.get_size() {
			Some(next_ptr as *mut FreeChunk)
		} else {
			None
		}
	}

	/// Splits the chunk with the given size `size` if necessary. The function might create a new
	/// chunk next to the current. The created chunk will be inserted in the free list but the
	/// current chunk will not.
	pub fn split(&mut self, size: usize) {
		#[cfg(config_debug_debug)]
		self.check();
		debug_assert!(self.get_size() >= size);

		if !self.is_used() {
			self.as_free_chunk().free_list_remove();
		}

		if let Some(next_ptr) = self.get_split_next_chunk(size) {
			let curr_new_size = (next_ptr as usize) - (self.get_ptr() as usize);
			let next_size = self.size - curr_new_size - size_of::<Chunk>();
			let next = unsafe { // Safe since `next_ptr` is valid
				ptr::write_volatile(next_ptr, FreeChunk::new(next_size));
				&mut *next_ptr
			};
			#[cfg(config_debug_debug)]
			next.check();
			next.free_list_insert();
			next.chunk.list.insert_after(&mut self.list);
			debug_assert!(!next.chunk.list.is_single());

			self.size = curr_new_size;
		}

		#[cfg(config_debug_debug)]
		self.check();
	}

	/// Tries to coalesce the chunk it with adjacent chunks if they are free. The function returns
	/// the resulting chunk, which will not be inserted into any free list.
	pub fn coalesce(&mut self) -> &mut Chunk {
		if !self.is_used() {
			self.as_free_chunk().free_list_remove();
		}

		if let Some(next) = self.list.get_next() {
			let n = next.get_mut::<Chunk>(crate::offset_of!(Chunk, list));

			if !n.is_used() {
				self.size += size_of::<Chunk>() + n.size;
				unsafe {
					next.unlink_floating();
				}
				n.as_free_chunk().free_list_remove();
				#[cfg(config_debug_debug)]
				n.check();
			}
		}

		if !self.is_used() {
			if let Some(prev) = self.list.get_prev() {
				let p = prev.get_mut::<Chunk>(crate::offset_of!(Chunk, list));
				if !p.is_used() {
					return p.coalesce();
				}
			}
		}

		#[cfg(config_debug_debug)]
		self.check();
		self
	}

	/// Tries to grow the given chunk of `delta` more bytes. If not possible, the function returns
	/// `false`. The function might alter the free list to get the space needed.
	pub fn grow(&mut self, delta: usize) -> bool {
		debug_assert!(self.is_used());
		debug_assert!(delta != 0);

		let next = self.list.get_next();
		if next.is_none() {
			return false;
		}
		let node = next.unwrap();
		let n = node.get_mut::<Chunk>(crate::offset_of!(Chunk, list));
		if n.is_used() {
			return false;
		}

		let new_size = self.size + delta;
		let available_size = size_of::<Chunk>() + n.size;
		if available_size < delta {
			return false;
		}
		self.size += available_size;

		unsafe {
			node.unlink_floating();
		}
		n.as_free_chunk().free_list_remove();

		self.split(new_size);
		#[cfg(config_debug_debug)]
		self.check();

		true
	}

	/// Tries to shrink the given chunk of `delta` less bytes. If not possible, the function
	/// returns `false`. The function might alter the free list to relinquish the space.
	pub fn shrink(&mut self, delta: usize) {
		debug_assert!(self.is_used());
		debug_assert!(delta != 0);
		debug_assert!(delta < self.get_size());

		let new_size = max(self.get_size() - delta, get_min_chunk_size());
		if self.get_split_next_chunk(new_size).is_some() {
			self.split(new_size);

			let next = self.list.get_next().unwrap();
			let n = next.get_mut::<Chunk>(crate::offset_of!(Chunk, list));
			debug_assert!(!n.is_used());
			n.coalesce();
		}

		#[cfg(config_debug_debug)]
		self.check();
	}
}

impl FreeChunk {
	/// Creates a new free with the given size `size` in bytes, meant to be the first chunk of a
	/// block. The chunk is **not** inserted into the free list.
	pub fn new_first(ptr: *mut c_void, size: usize) {
		unsafe {
			ptr::write_volatile(ptr as *mut FreeChunk, Self {
				chunk: Chunk {
					#[cfg(config_debug_malloc_magic)]
					magic: CHUNK_MAGIC,
					list: ListNode::new_single(),
					flags: 0,
					size,
				},
				free_list: ListNode::new_single(),
			});
		}
	}

	/// Creates a new free chunk. `size` is the size of the available memory in the chunk.
	pub fn new(size: usize) -> Self {
		Self {
			chunk: Chunk {
				#[cfg(config_debug_malloc_magic)]
				magic: CHUNK_MAGIC,
				list: ListNode::new_single(),
				flags: 0,
				size,
			},
			free_list: ListNode::new_single(),
		}
	}

	/// Returns a pointer to the chunks' data.
	pub fn get_ptr(&mut self) -> *mut c_void {
		self.chunk.get_ptr()
	}

	/// Returns a const pointer to the chunks' data.
	pub fn get_const_ptr(&self) -> *const c_void {
		self.chunk.get_const_ptr()
	}

	/// Returns the size of the chunk.
	pub fn get_size(&self) -> usize {
		self.chunk.get_size()
	}

	/// Checks that the chunk is correct. This function uses assertions and thus is useful only in
	/// debug mode.
	#[cfg(config_debug_debug)]
	pub fn check(&self) {
		debug_assert!(!self.chunk.is_used());
		self.chunk.check();
	}

	/// Returns the chunk object.
	pub fn get_chunk(&mut self) -> &mut Chunk {
		&mut self.chunk
	}

	/// Inserts the chunk into the appropriate free list.
	pub fn free_list_insert(&mut self) {
		#[cfg(config_debug_debug)]
		self.check();
		#[cfg(config_debug_debug)]
		check_free_lists();

		let free_list = get_free_list(self.get_size(), true).unwrap();
		free_list.insert_front(&mut self.free_list);

		#[cfg(config_debug_debug)]
		check_free_lists();
	}

	/// Removes the chunk from its free list.
	pub fn free_list_remove(&mut self) {
		#[cfg(config_debug_debug)]
		self.check();
		#[cfg(config_debug_debug)]
		check_free_lists();

		let free_list = get_free_list(self.get_size(), true).unwrap();
		self.free_list.unlink_from(free_list);

		#[cfg(config_debug_debug)]
		check_free_lists();
	}
}

/// Returns the minimum data size for a chunk.
fn get_min_chunk_size() -> usize {
	max(size_of::<FreeChunk>() - size_of::<Chunk>(), FREE_CHUNK_MIN)
}

/// Checks the chunks inside of each free lists.
#[cfg(config_debug_debug)]
fn check_free_lists() {
	let free_lists = unsafe {
		FREE_LISTS.assume_init_mut()
	};

	for free_list in free_lists {
		free_list.foreach(| node | {
			node.get::<FreeChunk>(crate::offset_of!(FreeChunk, free_list)).check();
		});
	}
}

/// Returns the free list for the given size `size`. If `insert` is not set, the function may
/// return a free list that contain chunks greater than the required size so that it can be split.
fn get_free_list(size: usize, insert: bool) -> Option<&'static mut FreeList> {
	#[cfg(config_debug_debug)]
	check_free_lists();

	let mut i = math::log2(size / FREE_LIST_SMALLEST_SIZE);
	i = min(i, FREE_LIST_BINS - 1);

	let free_lists = unsafe {
		FREE_LISTS.assume_init_mut()
	};

	if !insert {
		i += 1;

		while i < FREE_LIST_BINS && free_lists[i].is_empty() {
			i += 1;
		}

		if i >= FREE_LIST_BINS {
			return None;
		}
	}

	Some(&mut free_lists[i])
}

/// Initializes the free lists.
pub fn init_free_lists() {
	let free_lists = unsafe {
		FREE_LISTS.assume_init_mut()
	};

	for free_list in free_lists {
		*free_list = list_new!(FreeChunk, free_list);
	}
}

/// Returns a reference to a free chunk suitable for an allocation of given size `size`.
/// On success, the return value MUST be used or might result in a memory leak.
pub fn get_available_chunk(size: usize) -> Result<&'static mut FreeChunk, Errno> {
	let free_list = get_free_list(size, false);
	let chunk = {
		if let Some(f) = free_list {
			f.get_front().unwrap().get_mut(f.get_inner_offset())
		} else {
			let block = Block::new(size)?;
			unsafe {
				&mut *(&mut block.first_chunk as *mut _ as *mut FreeChunk)
			}
		}
	};

	#[cfg(config_debug_debug)]
	chunk.check();
	debug_assert!(chunk.get_size() >= size);

	Ok(chunk)
}
