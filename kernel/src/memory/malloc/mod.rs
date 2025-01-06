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

//! Implementation of the global memory allocator for kernelspace operations.

mod block;
mod chunk;

use crate::{memory, memory::malloc::ptr::NonNull, sync::mutex::IntMutex};
use block::Block;
use chunk::Chunk;
use core::{alloc::Layout, cmp::Ordering, intrinsics::unlikely, num::NonZeroUsize, ptr};
use utils::errno::AllocResult;

/// The allocator's mutex.
static MUTEX: IntMutex<()> = IntMutex::new(());

unsafe fn alloc(n: NonZeroUsize) -> AllocResult<NonNull<u8>> {
	let _ = MUTEX.lock();
	// Get free chunk
	let free_chunk = chunk::get_available_chunk(n)?;
	free_chunk.chunk.split(n.get());
	#[cfg(config_debug_malloc_check)]
	free_chunk.check();
	// Mark chunk as used
	let chunk = &mut free_chunk.chunk;
	chunk.used = true;
	// Return pointer
	let ptr = chunk.ptr();
	debug_assert!(ptr.is_aligned_to(chunk::ALIGNMENT));
	debug_assert!(ptr.as_ptr() as usize >= memory::PROCESS_END.0);
	#[cfg(feature = "memtrace")]
	super::trace::sample(
		"malloc",
		super::trace::SampleOp::Alloc,
		ptr.as_ptr() as usize,
		n.get(),
	);
	Ok(ptr)
}

unsafe fn realloc(ptr: NonNull<u8>, n: NonZeroUsize) -> AllocResult<NonNull<u8>> {
	let _ = MUTEX.lock();
	// Get chunk
	let chunk = Chunk::from_ptr(ptr.as_ptr());
	assert!(chunk.used);
	#[cfg(config_debug_malloc_check)]
	chunk.check();
	let chunk_size = chunk.get_size();
	let new_ptr = match n.get().cmp(&chunk_size) {
		Ordering::Less => {
			chunk.shrink(chunk_size - n.get());
			ptr
		}
		Ordering::Greater => {
			if !chunk.grow(n.get() - chunk_size) {
				// Allocate new chunk and copy to it
				let mut new_ptr = alloc(n)?;
				ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut(), chunk_size);
				free(ptr);
				new_ptr
			} else {
				ptr
			}
		}
		Ordering::Equal => ptr,
	};
	#[cfg(feature = "memtrace")]
	super::trace::sample(
		"malloc",
		super::trace::SampleOp::Realloc,
		ptr.as_ptr() as _,
		n.get(),
	);
	Ok(new_ptr)
}

unsafe fn free(mut ptr: NonNull<u8>) {
	let _ = MUTEX.lock();
	// Get chunk
	let chunk = Chunk::from_ptr(ptr.as_mut());
	assert!(chunk.used);
	#[cfg(config_debug_malloc_check)]
	chunk.check();
	// Mark as free
	chunk.used = false;
	let free_chunk = chunk.as_free_chunk().unwrap();
	free_chunk.prev = None;
	free_chunk.next = None;
	// Merge with adjacent chunks
	let chunk = chunk.coalesce();
	// If this is the last chunk in the block, free the block
	if chunk.is_single() {
		chunk.as_free_chunk().unwrap().free_list_remove();
		let block = Block::from_first_chunk(NonNull::from(chunk));
		block.drop_in_place();
	}
	#[cfg(feature = "memtrace")]
	super::trace::sample("malloc", super::trace::SampleOp::Free, ptr.as_ptr() as _, 0);
}

#[no_mangle]
unsafe fn __alloc(layout: Layout) -> AllocResult<NonNull<[u8]>> {
	let Some(size) = NonZeroUsize::new(layout.size()) else {
		return Ok(NonNull::slice_from_raw_parts(layout.dangling(), 0));
	};
	let ptr = alloc(size)?;
	Ok(NonNull::slice_from_raw_parts(ptr, size.get()))
}

#[no_mangle]
unsafe fn __realloc(
	ptr: NonNull<u8>,
	old_layout: Layout,
	new_layout: Layout,
) -> AllocResult<NonNull<[u8]>> {
	let Some(new_size) = NonZeroUsize::new(new_layout.size()) else {
		__dealloc(ptr, old_layout);
		return Ok(NonNull::slice_from_raw_parts(new_layout.dangling(), 0));
	};
	let ptr = realloc(ptr, new_size)?;
	Ok(NonNull::slice_from_raw_parts(ptr, new_size.get()))
}

#[no_mangle]
unsafe fn __dealloc(ptr: NonNull<u8>, layout: Layout) {
	if unlikely(layout.size() == 0) {
		return;
	}
	free(ptr);
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::memory::buddy;
	use core::slice;
	use utils::{limits::PAGE_SIZE, math};

	#[test_case]
	fn alloc_free1() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let ptr = alloc(NonZeroUsize::new(1).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr(), 1).fill(!0);
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free1() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let ptr = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr(), 8).fill(!0);
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free2() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let ptr = alloc(NonZeroUsize::new(PAGE_SIZE).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr(), PAGE_SIZE).fill(!0);
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free3() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let ptr = alloc(NonZeroUsize::new(PAGE_SIZE * 10).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr(), PAGE_SIZE * 10).fill(!0);
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free_fifo() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let mut ptrs: [NonNull<u8>; 1024] = [NonNull::dangling(); 1024];
			for (i, p) in ptrs.iter_mut().enumerate() {
				let size = i + 1;
				let ptr = alloc(NonZeroUsize::new(size).unwrap()).unwrap();
				slice::from_raw_parts_mut(ptr.as_ptr(), size).fill(!0);
				*p = ptr;
			}
			for i in 0..ptrs.len() {
				for j in (i + 1)..ptrs.len() {
					assert_ne!(ptrs[j], ptrs[i]);
				}
			}
			for p in ptrs {
				free(p);
			}
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	fn lifo_test(i: usize) {
		unsafe {
			let ptr = alloc(NonZeroUsize::new(i).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr(), i).fill(!0);
			if i > 1 {
				lifo_test(i - 1);
			}
			free(ptr);
		}
	}

	#[test_case]
	fn alloc_free_lifo() {
		let usage = buddy::allocated_pages_count();
		lifo_test(100);
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	// TODO Check the integrity of the data after reallocation
	#[test_case]
	fn realloc0() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let mut ptr = alloc(NonZeroUsize::new(1).unwrap()).unwrap();
			for i in 1..PAGE_SIZE {
				ptr = realloc(ptr, NonZeroUsize::new(i).unwrap()).unwrap();
				slice::from_raw_parts_mut(ptr.as_ptr(), i).fill(!0);
			}
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	// TODO Check the integrity of the data after reallocation
	#[test_case]
	fn realloc1() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let mut ptr = alloc(NonZeroUsize::new(PAGE_SIZE).unwrap()).unwrap();
			for i in (1..PAGE_SIZE).rev() {
				ptr = realloc(ptr, NonZeroUsize::new(i).unwrap()).unwrap();
				slice::from_raw_parts_mut(ptr.as_ptr(), i).fill(!0);
			}
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	// TODO Check the integrity of the data after reallocation
	#[test_case]
	fn realloc2() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let mut ptr0 = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr0.as_ptr(), 8).fill(!0);
			let mut ptr1 = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr1.as_ptr(), 8).fill(!0);
			for i in 0..8 {
				ptr0 = realloc(ptr0, NonZeroUsize::new(math::pow2(i)).unwrap()).unwrap();
				ptr1 = realloc(ptr1, NonZeroUsize::new(math::pow2(i) + 1).unwrap()).unwrap();
			}
			free(ptr1);
			free(ptr0);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	// TODO Check the integrity of the data after reallocation
	#[test_case]
	fn realloc3() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let mut ptr0 = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr0.as_ptr(), 8).fill(!0);
			let mut ptr1 = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr1.as_ptr(), 8).fill(!0);
			for i in (0..8).rev() {
				ptr0 = realloc(ptr0, NonZeroUsize::new(math::pow2(i)).unwrap()).unwrap();
				ptr1 = realloc(ptr1, NonZeroUsize::new(math::pow2(i) + 1).unwrap()).unwrap();
			}
			free(ptr1);
			free(ptr0);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}
}
