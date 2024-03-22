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

//! This module implements the global memory allocator for kernelside operations.
//!
//! The allocator is accessible through [`alloc::alloc::Global`].

mod block;
mod chunk;

use crate::{memory, memory::malloc::ptr::NonNull};
use block::Block;
use chunk::Chunk;
use core::{
	alloc::{AllocError, GlobalAlloc, Layout},
	cmp::Ordering,
	num::NonZeroUsize,
	ptr,
	ptr::{drop_in_place, null_mut},
};
use macros::instrument_allocator;
use utils::{errno::AllocResult, lock::IntMutex};

/// The allocator's mutex.
static MUTEX: IntMutex<()> = IntMutex::new(());

#[instrument_allocator(name = malloc, op = alloc, size = n)]
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
	let ptr = chunk.get_ptr_mut();
	debug_assert!(ptr.is_aligned_to(chunk::ALIGNMENT));
	debug_assert!(ptr as usize >= memory::PROCESS_END as usize);
	NonNull::new(ptr).ok_or(AllocError)
}

#[instrument_allocator(name = malloc, op = realloc, ptr = ptr, size = n)]
unsafe fn realloc(ptr: NonNull<u8>, n: NonZeroUsize) -> AllocResult<NonNull<u8>> {
	let _ = MUTEX.lock();
	// Get chunk
	let chunk = Chunk::from_ptr(ptr.as_ptr());
	assert!(chunk.used);
	#[cfg(config_debug_malloc_check)]
	chunk.check();
	let chunk_size = chunk.get_size();
	match n.get().cmp(&chunk_size) {
		Ordering::Less => {
			chunk.shrink(chunk_size - n.get());
			Ok(ptr)
		}
		Ordering::Greater => {
			if !chunk.grow(n.get() - chunk_size) {
				// Allocate new chunk and copy to it
				let mut new_ptr = alloc(n)?;
				ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut(), chunk_size);
				free(ptr);
				Ok(new_ptr)
			} else {
				Ok(ptr)
			}
		}
		Ordering::Equal => Ok(ptr),
	}
}

#[instrument_allocator(name = malloc, op = free, ptr = ptr)]
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
	if chunk.is_single() {
		chunk.as_free_chunk().unwrap().free_list_remove();
		let block = Block::from_first_chunk(chunk);
		drop_in_place(block);
	}
}

/// The global allocator for the kernel.
struct Malloc;

unsafe impl GlobalAlloc for Malloc {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		let Some(size) = NonZeroUsize::new(layout.size()) else {
			return null_mut();
		};
		alloc(size).map(|p| p.as_ptr()).unwrap_or(null_mut())
	}

	unsafe fn dealloc(&self, ptr: *mut u8, _: Layout) {
		let Some(ptr) = NonNull::new(ptr) else {
			return;
		};
		free(ptr);
	}

	unsafe fn realloc(&self, ptr: *mut u8, _: Layout, new_size: usize) -> *mut u8 {
		let Some(ptr) = NonNull::new(ptr) else {
			return null_mut();
		};
		match NonZeroUsize::new(new_size) {
			Some(new_size) => realloc(ptr, new_size)
				.map(|p| p.as_ptr())
				.unwrap_or(null_mut()),
			None => {
				free(ptr);
				null_mut()
			}
		}
	}
}

#[global_allocator]
static ALLOCATOR: Malloc = Malloc;

#[cfg(test)]
mod test {
	use super::*;
	use crate::{memory, memory::buddy};
	use core::slice;
	use utils::math;

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
			let ptr = alloc(NonZeroUsize::new(memory::PAGE_SIZE).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr(), memory::PAGE_SIZE).fill(!0);
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free3() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let ptr = alloc(NonZeroUsize::new(memory::PAGE_SIZE * 10).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr(), memory::PAGE_SIZE * 10).fill(!0);
			free(ptr);
		}
		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free_fifo() {
		let usage = buddy::allocated_pages_count();
		unsafe {
			let mut ptrs: [NonNull<u8>; 1024] = [NonNull::new(1 as _).unwrap(); 1024];
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
			for i in 1..memory::PAGE_SIZE {
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
			let mut ptr = alloc(NonZeroUsize::new(memory::PAGE_SIZE).unwrap()).unwrap();
			for i in (1..memory::PAGE_SIZE).rev() {
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
