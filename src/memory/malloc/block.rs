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

//! In the malloc allocator, a block is a memory allocation performed from
//! another allocator, which is too big to be used directly for allocation, so
//! it has to be divided into chunks.

use super::chunk::Chunk;
use super::chunk::FreeChunk;
use crate::errno::AllocResult;
use crate::memory;
use crate::memory::buddy;
use crate::util::math;
use core::mem::offset_of;
use core::mem::size_of;
use core::num::NonZeroUsize;
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
	pub fn new(min_size: NonZeroUsize) -> AllocResult<&'static mut Self> {
		let min_total_size = size_of::<Block>() + min_size.get();
		let block_order = buddy::get_order(math::ceil_div(min_total_size, memory::PAGE_SIZE));

		// The size of the first chunk
		let first_chunk_size = buddy::get_frame_size(block_order) - size_of::<Block>();
		debug_assert!(first_chunk_size >= min_size.get());

		// Allocate the block
		let mut ptr = buddy::alloc_kernel(block_order)?.cast();
		// Init block
		unsafe {
			ptr::write_volatile(
				ptr.as_mut(),
				Self {
					order: block_order,
					first_chunk: Chunk::new(),
				},
			);
		}

		let block = unsafe { ptr.as_mut() };
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
