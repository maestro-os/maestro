//! In the malloc allocator, a block is a memory allocation performed from another allocator, which
//! is too big to be used directly for allocation, so it has to be divided into chunks.

use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use crate::errno::Errno;
use crate::memory::buddy;
use crate::memory;
use crate::offset_of;
use crate::util::list::ListNode;
use crate::util::math;
use crate::util;
use super::chunk::Chunk;
use super::chunk::FreeChunk;

/// Structure representing a frame of memory allocated using the buddy allocator, storing memory
/// chunks.
#[repr(C, align(8))]
pub struct Block {
	/// The linked list storing the blocks
	list: ListNode,
	/// The order of the frame for the buddy allocator
	order: buddy::FrameOrder,
	/// The first chunk of the block
	pub first_chunk: Chunk,
}

impl Block {
	/// Allocates a new block of memory with the minimum available size `min_size` in bytes.
	/// The buddy allocator must be initialized before using this function.
	/// The underlying chunk created by this function is **not** inserted into the free list.
	pub fn new(min_size: usize) -> Result<&'static mut Self, Errno> {
		let total_min_size = size_of::<Block>() + min_size;
		let order = buddy::get_order(math::ceil_division(total_min_size, memory::PAGE_SIZE));
		let first_chunk_size = buddy::get_frame_size(order) - size_of::<Block>();
		debug_assert!(first_chunk_size >= min_size);

		let ptr = buddy::alloc_kernel(order)?;
		let block = unsafe { // Safe since `ptr` is valid
			ptr::write_volatile(ptr as *mut Block, Self {
				list: ListNode::new_single(),
				order,
				first_chunk: Chunk::new(),
			});
			&mut *(ptr as *mut Block)
		};
		FreeChunk::new_first(&mut block.first_chunk as *mut _ as *mut c_void, first_chunk_size);
		Ok(block)
	}

	/// Returns a mutable reference to the block whose first chunk's reference is passed as argument.
	pub unsafe fn from_first_chunk(chunk: *mut Chunk) -> &'static mut Block {
		let first_chunk_off = offset_of!(Block, first_chunk);
		let ptr = ((chunk as usize) - first_chunk_off) as *mut Self;
		debug_assert!(util::is_aligned(ptr as *const c_void, memory::PAGE_SIZE));
		&mut *ptr
	}

	/// Returns the total size of the block in bytes.
	fn get_total_size(&self) -> usize {
		buddy::get_frame_size(self.order)
	}
}

impl Drop for Block {
	fn drop(&mut self) {
		buddy::free_kernel(self as *mut _ as _, self.order);
	}
}
