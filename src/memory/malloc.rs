/*
 * This file handles allocations of chunks of kernel memory.
 *
 * TODO: More documentation
 */

use core::cmp::{min, max};
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::mem::size_of;
use crate::memory::PAGE_SIZE;
use crate::memory::buddy;
use crate::util::data_struct::LinkedList;
use crate::util;

/*
 * Type representing chunks' flags.
 */
type ChunkFlags = u8;

/* Chunk flag indicating that the chunk is being used */
const CHUNK_FLAG_USED: ChunkFlags = 0b1;

/*
 * The minimum amount of bytes required to create a free chunk.
 */
const FREE_CHUNK_MIN: usize = 8;

/*
 * The size of the smallest free list bin.
 */
const FREE_LIST_SMALLEST_SIZE: usize = FREE_CHUNK_MIN;

/*
 * The number of free list bins.
 */
const FREE_LIST_BINS: usize = 8;

/*
 * A chunk of allocated or free memory stored in linked lists.
 */
struct Chunk {
	/* The linked list storing the chunks */
	list: LinkedList,
	/* The chunk's flags */
	flags: u8,
	/* The size of the chunk's memory in bytes */
	size: usize,
}

/*
 * A free chunk, wrapping the Chunk structure.
 */
#[repr(C)]
struct FreeChunk {
	/* The chunk */
	chunk: Chunk,
	/* The linked list for the free list */
	free_list: LinkedList,
}

/*
 * Structure representing a frame of memory allocated using the buddy allocator, storing memory
 * chunks.
 */
struct Block {
	/* The linked list storing the blocks */
	list: LinkedList,
	/* The order of the frame for the buddy allocator */
	order: buddy::FrameOrder,
	/* The first chunk of the block */
	first_chunk: Chunk,
}

/*
 * Type representing a free list entry into the free lists list
 */
type FreeList = Option<*mut LinkedList>;

/*
 * List storing allocated blocks of memory.
 */
static mut FREE_LISTS: MaybeUninit<[FreeList; FREE_LIST_BINS]> = MaybeUninit::uninit();

/*
 * Returns the free list for the given size `size`. If `insert` is not set, the function may return
 * a free list that contain chunks greater than the required size so that it can be split.
 */
fn get_free_list(size: usize, insert: bool) -> Option<&'static mut FreeList> {
	let mut i = util::log2(size / FREE_LIST_SMALLEST_SIZE);
	if i >= FREE_LIST_BINS {
		i = FREE_LIST_BINS - 1;
	}

	let free_lists = unsafe {
		FREE_LISTS.assume_init_mut()
	};

	if !insert {
		while i < FREE_LIST_BINS && free_lists[i].is_none() {
			i += 1;
		}

		if i >= FREE_LIST_BINS {
			return None;
		}
	}

	Some(&mut free_lists[i])
}

impl Chunk {
	/*
	 * Returns the chunk corresponding to the given data pointer.
	 */
	pub unsafe fn from_ptr(ptr: *mut c_void) -> &'static mut Self {
		&mut *(ptr.sub(core::mem::size_of::<Self>()) as *mut Self)
	}

	/*
	 * Tells the whether the chunk is free.
	 */
	pub fn is_used(&self) -> bool {
		(self.flags & CHUNK_FLAG_USED) != 0
	}

	/*
	 * Returns a pointer to the chunks' data.
	 */
	pub fn get_ptr(&mut self) -> *mut c_void {
		unsafe {
			(self as *mut Self as *mut c_void).add(core::mem::size_of::<Self>())
		}
	}

	/*
	 * Returns the size of the chunk.
	 */
	pub fn get_size(&self) -> usize {
		self.size
	}

	/*
	 * Returns a mutable reference for the given chunk as a free chunk. The result is undefined if
	 * the chunk is used.
	 */
	pub fn as_free_chunk(&mut self) -> &mut FreeChunk {
		debug_assert!(!self.is_used());
		unsafe {
			&mut *(self as *mut Self as *mut FreeChunk)
		}
	}

	/*
	 * Marks the chunk as free and tries to coalesce it with adjacent chunks if they are free.
	 * The function returns the resulting free chunk.
	 */
	pub fn coalesce(&mut self) -> &mut FreeChunk {
		self.flags &= CHUNK_FLAG_USED;

		if let Some(next) = self.list.get_next() {
			let n = unsafe {
				&*crate::linked_list_get!(next as *mut LinkedList, *const Chunk, list)
			};

			if !n.is_used() {
				self.size += core::mem::size_of::<Chunk>() + n.size;
				next.unlink_floating();
				// TODO Unlink from free list
			}
		}

		if let Some(prev) = self.list.get_prev() {
			let p = unsafe {
				&mut *crate::linked_list_get!(prev as *mut LinkedList, *mut Chunk, list)
			};

			if !p.is_used() {
				p.coalesce();
			}
			p.as_free_chunk()
		} else {
			self.as_free_chunk()
		}
	}

	/*
	 * Tries to resize the chunk, adding `delta` bytes. A negative number of bytes results in chunk
	 * shrinking. Returns `true` if possible, or `false` if not. If the chunk cannot be expanded,
	 * the function does nothing. Expansion might reduce/move/remove the next chunk if it is free.
	 * If `delta` is zero, the function returns `false`.
	 */
	pub fn resize(&mut self, delta: isize) -> bool {
		if delta == 0 {
			return true;
		}

		let mut valid = false;

		if delta > 0 {
			if let Some(next) = self.list.get_next() {
				let n = unsafe {
					&mut *crate::linked_list_get!(next as *mut LinkedList, *mut Chunk, list)
				};

				if n.is_used() {
					return false;
				}

				let available_size = core::mem::size_of::<Chunk>() + n.size;
				if available_size < delta as usize {
					return false;
				}

				next.unlink_floating();
				n.as_free_chunk().free_list_remove();

				let next_min_size = core::mem::size_of::<Chunk>() + FREE_CHUNK_MIN;
				if available_size - delta as usize >= next_min_size {
					// TODO Move next chunk (relink to both list and free list)
				}

				valid = true;
			}
		}

		if delta < 0 {
			if self.size <= (-delta) as usize {
				return false;
			}

			if let Some(next) = self.list.get_next() {
				let n = unsafe {
					&*crate::linked_list_get!(next as *mut LinkedList, *const Chunk, list)
				};

				if !n.is_used() {
					// TODO Move next chunk
				}
			}

			valid = true;
		}

		if valid {
			if delta >= 0 {
				self.size += delta as usize;
			} else {
				self.size -= delta.abs() as usize;
			}
		}
		valid
	}
}

impl FreeChunk {
	/*
	 * Creates a new free with the given size `size` in bytes, meant to be the first chunk of a
	 * block.
	 */
	pub fn new_first(ptr: *mut c_void, size: usize) {
		let c = unsafe {
			&mut *(ptr as *mut Self)
		};
		*c = Self {
			chunk: Chunk {
				list: LinkedList::new_single(),
				flags: 0,
				size: size,
			},
			free_list: LinkedList::new_single(),
		};
		c.free_list_insert();
	}

	/*
	 * Returns the chunk corresponding to the given data pointer.
	 */
	pub unsafe fn from_ptr(ptr: *mut c_void) -> &'static mut Self {
		&mut *(ptr.sub(core::mem::size_of::<Self>()) as *mut Self)
	}

	/*
	 * Returns a pointer to the chunks' data.
	 */
	pub fn get_ptr(&mut self) -> *mut c_void {
		self.chunk.get_ptr()
	}

	/*
	 * Returns the size of the chunk.
	 */
	pub fn get_size(&self) -> usize {
		self.chunk.get_size()
	}

	/*
	 * Checks that the chunk is correct. This function uses assertions and thus is useful only in
	 * debug mode.
	 */
	pub fn check(&self) {
		debug_assert!(!self.chunk.is_used());
	}

	/*
	 * Tells whether the chunk can be split for the given size `size`.
	 */
	fn can_split(&self, size: usize) -> bool {
		let min_data_size = max(size_of::<FreeChunk>() - size_of::<Chunk>(), FREE_CHUNK_MIN);
		self.get_size() >= size + size_of::<Chunk>() + min_data_size
	}

	/*
	 * Splits the chunk with the given size `size` if necessary and marks it as used. The function
	 * might create a new chunk next to the current.
	 */
	pub fn split(&mut self, size: usize) {
		debug_assert!(self.get_size() >= size);

		if self.can_split(size) {
			let next_off = (self as *mut Self as usize) + core::mem::size_of::<Chunk>() + size;
			let next = unsafe {
				&mut *(next_off as *mut FreeChunk)
			};
			util::zero_object(next);
			next.chunk.flags = 0;
			next.chunk.size = self.get_size() - (size + core::mem::size_of::<Chunk>());
			next.chunk.list.insert_after(&mut self.chunk.list);
			next.free_list_insert();
		}

		self.chunk.flags |= CHUNK_FLAG_USED;
	}

	/*
	 * Inserts the chunk into the appropriate free list.
	 */
	pub fn free_list_insert(&mut self) {
		let free_list = get_free_list(self.chunk.size, true).unwrap();
		self.free_list.insert_front(free_list);
	}

	/*
	 * Removes the chunk from its free list.
	 */
	pub fn free_list_remove(&mut self) {
		let free_list = get_free_list(self.chunk.size, false).unwrap();
		self.free_list.unlink(free_list);
	}
}

impl Block {
	/*
	 * Allocates a new block of memory with the minimum available size `min_size` in bytes.
	 * The buddy allocator must be initialized before using this function.
	 */
	fn new(min_size: usize) -> Result<&'static mut Self, ()> {
		let total_min_size = core::mem::size_of::<Block>() + min_size;
		let order = buddy::get_order(util::ceil_division(total_min_size, PAGE_SIZE));
		let first_chunk_size = buddy::get_frame_size(order) - core::mem::size_of::<Block>();

		let ptr = buddy::alloc_kernel(order)?;
		let block = unsafe { &mut *(ptr as *mut Block) };
		*block = Self {
			list: LinkedList::new_single(),
			order: order,
			first_chunk: Chunk {
				list: LinkedList::new_single(),
				flags: 0,
				size: 0,
			},
		};
		FreeChunk::new_first(&mut block.first_chunk as *mut _ as *mut c_void, first_chunk_size);
		Ok(block)
	}

	/*
	 * Returns the total size of the block in bytes.
	 */
	fn get_total_size(&self) -> usize {
		buddy::get_frame_size(self.order)
	}
}

/*
 * Initializes the allocator. This function must be called before using the allocator's functions
 * and exactly once.
 */
pub fn init() {
	unsafe {
		util::zero_object(&mut FREE_LISTS);
	}
}

// TODO Mutex
/*
 * Allocates `n` bytes of kernel memory and returns a pointer to the beginning of the allocated
 * chunk. If the allocation fails, the function shall return an error.
 */
pub fn alloc(n: usize) -> Result<*mut c_void, ()> {
	let mut f = get_free_list(n, false);
	if f.is_none() {
		if Block::new(n).is_err() {
			return Err(());
		}

		f = get_free_list(n, false);
		debug_assert!(f.is_some());
	}

	let free_list = f.unwrap();
	let chunk_node = unsafe {
		&mut *free_list.unwrap()
	};
	let chunk = unsafe {
		&mut *crate::linked_list_get!(chunk_node, *mut FreeChunk, free_list)
	};
	chunk.check();
	debug_assert!(chunk.get_size() >= n);
	chunk.split(n);
	Ok(chunk.get_ptr())
}

// TODO Mutex
/*
 * Changes the size of the memory previously allocated with `alloc`. `ptr` is the pointer to the
 * chunk of memory. `n` is the new size of the chunk of memory. If the reallocation fails, the
 * chunk is left untouched and the function returns an error.
 */
pub fn realloc(ptr: *mut c_void, n: usize) -> Result<*mut c_void, ()> {
	let chunk = unsafe {
		Chunk::from_ptr(ptr)
	};
	// TODO Check that chunk is valid?

	if !chunk.resize((n as isize) - (chunk.get_size() as isize)) {
		let new_ptr = alloc(n)?;
		unsafe {
			util::memcpy(new_ptr, ptr, min(chunk.get_size(), n));
		}
		free(ptr);
		Ok(new_ptr)
	} else {
		Ok(ptr)
	}
}

// TODO Mutex
/*
 * Frees the memory at the pointer `ptr` previously allocated with `alloc`. Subsequent uses of the
 * associated memory are undefined.
 */
pub fn free(ptr: *mut c_void) {
	let chunk = unsafe {
		FreeChunk::from_ptr(ptr)
	};
	// TODO Check that chunk is valid?

	let c = chunk.chunk.coalesce();
	// TODO Remove block if contains only one free chunk
	// TODO else:
	c.free_list_insert();
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn alloc_free0() {
		if let Ok(ptr) = alloc(1) {
			unsafe {
				util::memset(ptr, -1, 1);
			}
			free(ptr);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn alloc_free1() {
		if let Ok(ptr) = alloc(8) {
			unsafe {
				util::memset(ptr, -1, 8);
			}
			free(ptr);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn alloc_free2() {
		if let Ok(ptr) = alloc(PAGE_SIZE) {
			unsafe {
				util::memset(ptr, -1, PAGE_SIZE);
			}
			free(ptr);
		} else {
			assert!(false);
		}
	}

	// TODO
}
