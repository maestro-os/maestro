/*
 * This file handles allocations of chunks of kernel memory.
 */

use core::ffi::c_void;
use crate::memory::PAGE_SIZE;
use crate::memory::buddy;
use crate::util::data_struct;
use crate::util;

type ChunkFlags = u8;

/* Chunk flag indicating that the chunk is being used */
const CHUNK_FLAG_USED: ChunkFlags = 0b1;

/*
 * A chunk of allocated or free memory stored in linked lists.
 */
struct Chunk {
	/* The linked list storing the chunks */
	list: data_struct::LinkedList,
	/* The chunk's flags */
	flags: u8,
	/* The size of the chunk's memory in bytes */
	size: usize,
}

/*
 * Structure representing a frame of memory allocated using the buddy allocator, storing memory
 * chunks.
 */
struct Block {
	/* The order of the frame for the buddy allocator */
	order: buddy::FrameOrder,
	/* The first chunk of the block */
	first_chunk: Chunk,
}

impl Chunk {
	/*
	 * Tells the whether the chunk is free.
	 */
	fn is_used(&self) -> bool {
		(self.flags & CHUNK_FLAG_USED) != 0
	}

	/*
	 * Tells whether the chunk can be split for the given size `size`.
	 */
	fn can_split(&self, _size: usize) -> bool {
		// TODO
		false
	}

	/*
	 * Splits the chunk with the given size `size` if necessary and marks it as used. The function
	 * might create a new chunk next to the current.
	 */
	fn split(&mut self, _size: usize) {
		// TODO
	}

	/*
	 * Marks the chunk as free and tries to coalesce it with adjacent chunks if they are free.
	 */
	fn coalesce(&mut self) {
		// TODO
	}
}

impl Block {
	/*
	 * Allocates a new block of memory with the minimum available size `min_size` in bytes.
	 */
	fn new(min_size: usize) -> Result<&'static mut Self, ()> {
		let total_min_size = core::mem::size_of::<Block>() + min_size;
		let order = buddy::get_order(util::ceil_division(total_min_size, PAGE_SIZE));

		let ptr = buddy::alloc_kernel(order)?;
		let block = unsafe { &mut *(ptr as *mut Block) };
		block.order = order;
		// TODO Init first_chunk
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
 * Allocates `n` bytes of kernel memory and returns a pointer to the beginning of the allocated
 * chunk. If the allocation fails, the function shall return None.
 */
pub fn alloc(_n: usize) -> Option<*mut c_void> {
	// TODO
	None
}

/*
 * Changes the size of the memory previously allocated with `alloc`. `ptr` is the pointer to the
 * chunk of memory. `n` is the new size of the chunk of memory. If the reallocation fails, the
 * chunk is left untouched.
 */
pub fn realloc(_ptr: *const c_void, _n: usize) -> Option<*mut c_void> {
	// TODO
	None
}

/*
 * Frees the memory at the pointer `ptr` previously allocated with `alloc`. Subsequent uses of the
 * associated memory are undefined.
 */
pub fn free(_ptr: *const c_void) {
	// TODO
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn alloc_free0() {
		if let Some(ptr) = alloc(1) {
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
		if let Some(ptr) = alloc(8) {
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
		if let Some(ptr) = alloc(PAGE_SIZE) {
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
