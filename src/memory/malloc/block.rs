//! In the malloc allocator, a block is a memory allocation performed from
//! another allocator, which is too big to be used directly for allocation, so
//! it has to be divided into chunks.

use super::chunk::Chunk;
use super::chunk::FreeChunk;
use crate::errno::Errno;
use crate::memory;
use crate::memory::buddy;
use crate::util::math;
use core::mem::offset_of;
use core::mem::size_of;
use core::ptr;

/// Structure representing a frame of memory allocated using the buddy
/// allocator, storing memory chunks.
#[repr(C, align(8))]
pub struct Block {
	/// The order of the frame for the buddy allocator
	order: buddy::FrameOrder,

	/// The first chunk of the block
	pub first_chunk: Chunk,
}

impl Block {
	/// Allocates a new block of memory with the minimum available size
	/// `min_size` in bytes.
	///
	/// The buddy allocator must be initialized before using this function.
	///
	/// The underlying chunk created by this function is **not** inserted into the free list.
	pub fn new(min_size: usize) -> Result<&'static mut Self, Errno> {
		let min_total_size = size_of::<Block>() + min_size;
		let block_order = buddy::get_order(math::ceil_div(min_total_size, memory::PAGE_SIZE));

		// The size of the first chunk
		let first_chunk_size = buddy::get_frame_size(block_order) - size_of::<Block>();
		debug_assert!(first_chunk_size >= min_size);

		// Allocate the block
		let ptr = buddy::alloc_kernel(block_order)? as *mut Block;
		// Init block
		unsafe {
			ptr::write_volatile(
				ptr,
				Self {
					order: block_order,
					first_chunk: Chunk::new(),
				},
			);
		}

		let block = unsafe { &mut *ptr };
		*block.first_chunk.as_free_chunk().unwrap() = FreeChunk::new(first_chunk_size);

		Ok(block)
	}

	/// Returns a mutable reference to the block whose first chunk's reference
	/// is passed as argument.
	pub unsafe fn from_first_chunk(chunk: *mut Chunk) -> &'static mut Block {
		let first_chunk_off = offset_of!(Block, first_chunk);
		let ptr = ((chunk as usize) - first_chunk_off) as *mut Self;
		debug_assert!(ptr.is_aligned_to(memory::PAGE_SIZE));

		&mut *ptr
	}

	/// Returns the total size of the block in bytes.
	#[inline]
	fn get_total_size(&self) -> usize {
		buddy::get_frame_size(self.order)
	}
}

impl Drop for Block {
	fn drop(&mut self) {
		buddy::free_kernel(self as *mut _ as _, self.order);
	}
}
