//! This module implements the memory allocation utility for kernelside
//! operations.
//!
//! An unsafe interface is provided, inspired from the C language's
//! malloc interface.
//!
//! The module also provides the structure `Alloc` which safely manages a memory allocation.

mod block;
mod chunk;

use crate::errno::AllocError;
use crate::errno::AllocResult;
use crate::memory;
use crate::memory::malloc::ptr::NonNull;
use crate::util::lock::IntMutex;
use block::Block;
use chunk::Chunk;
use core::cmp::min;
use core::cmp::Ordering;
use core::ffi::c_void;
use core::mem::size_of;
use core::num::NonZeroUsize;
use core::ops::Index;
use core::ops::IndexMut;
use core::ptr;
use core::ptr::drop_in_place;
use core::slice;

/// The allocator's mutex.
static MUTEX: IntMutex<()> = IntMutex::new(());

/// Allocates `n` bytes of kernel memory and returns a pointer to the beginning
/// of the allocated chunk.
///
/// If the allocation fails, the function returns an error.
///
/// The allocated memory is **not** initialized, meaning it may contain garbage, or even
/// sensitive informations.
/// It is the caller's responsibility to ensure the chunk of memory is correctly initialized.
///
/// # Safety
///
/// Allocated pointer must always be freed. Failure to do so results in a memory
/// leak. Writing outside of the allocated range (buffer overflow) results in an
/// undefined behaviour.
pub unsafe fn alloc(n: NonZeroUsize) -> AllocResult<NonNull<c_void>> {
	let _ = MUTEX.lock();

	let free_chunk = chunk::get_available_chunk(n)?;
	free_chunk.chunk.split(n.get());

	#[cfg(config_debug_malloc_check)]
	free_chunk.check();

	let chunk = &mut free_chunk.chunk;
	chunk.used = true;

	let ptr = chunk.get_ptr_mut();
	debug_assert!(ptr.is_aligned_to(chunk::ALIGNEMENT));
	debug_assert!(ptr as usize >= memory::PROCESS_END as usize);

	NonNull::new(ptr).ok_or(AllocError)
}

/// Changes the size of the memory previously allocated with `alloc`. `ptr` is
/// the pointer to the chunk of memory.
///
/// The allocated memory is **not** initialized.
///
/// `n` is the new size of the chunk of memory.
///
/// If the reallocation fails, the chunk is left untouched and the function
/// returns an error.
pub unsafe fn realloc(ptr: NonNull<c_void>, n: NonZeroUsize) -> AllocResult<NonNull<c_void>> {
	let _ = MUTEX.lock();

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
				let old_len = min(chunk.get_size(), n.get());
				let mut new_ptr = alloc(n)?;

				ptr::copy_nonoverlapping(ptr.as_ptr(), new_ptr.as_mut(), old_len);

				free(ptr);

				Ok(new_ptr)
			} else {
				Ok(ptr)
			}
		}

		Ordering::Equal => Ok(ptr),
	}
}

/// Frees the memory at the pointer `ptr` previously allocated with `alloc`.
///
/// # Safety
///
/// If `ptr` doesn't point to a valid chunk of memory allocated with the `alloc`
/// function, the behaviour is undefined.
///
/// Using memory after it was freed causes an undefined behaviour.
pub unsafe fn free(mut ptr: NonNull<c_void>) {
	let _ = MUTEX.lock();

	let chunk = Chunk::from_ptr(ptr.as_mut());
	assert!(chunk.used);

	#[cfg(config_debug_malloc_check)]
	chunk.check();

	chunk.used = false;
	let free_chunk = chunk.as_free_chunk().unwrap();
	free_chunk.prev = None;
	free_chunk.next = None;

	let chunk = chunk.coalesce();
	if chunk.is_single() {
		chunk.as_free_chunk().unwrap().free_list_remove();

		let block = Block::from_first_chunk(chunk);
		drop_in_place(block);
	}
}

/// Structure representing a kernelside allocation.
///
/// The structure holds one or more elements of the given type. Freeing the
/// allocation doesn't call `drop` on its elements.
#[derive(Debug)]
pub struct Alloc<T> {
	/// Slice representing the allocation.
	slice: NonNull<[T]>,
}

impl<T> Alloc<T> {
	/// Allocates `size` element in the kernel memory and returns a structure
	/// wrapping a slice allowing to access it.
	///
	/// If the allocation fails, the function shall return an error.
	///
	/// The allocated memory is **not** initialized, meaning it may contain garbage, or even
	/// sensitive informations.
	///
	/// It is the caller's responsibility to ensure the chunk of memory is correctly initialized.
	///
	/// # Safety
	///
	/// Since the memory is not initialized, objects in the allocation might be
	/// in an inconsistent state.
	pub unsafe fn new(size: NonZeroUsize) -> AllocResult<Self> {
		let len = size
			.checked_mul(size_of::<T>().try_into().unwrap())
			.ok_or(AllocError)?;
		let ptr = alloc(len)?;
		let slice = NonNull::new(slice::from_raw_parts_mut::<T>(
			ptr.cast().as_mut(),
			size.get(),
		))
		.unwrap();

		Ok(Self {
			slice,
		})
	}

	/// Same as `new`, except the memory chunk is zero-ed.
	///
	/// # Safety
	///
	/// Since the memory is zero-ed, objects in the allocation might be in an
	/// inconsistent state.
	pub unsafe fn new_zero(size: NonZeroUsize) -> AllocResult<Self> {
		let mut alloc = Self::new(size)?;

		// Zero memory
		let slice = slice::from_raw_parts_mut(alloc.as_ptr_mut() as *mut u8, size.get());
		slice.fill(0);

		Ok(alloc)
	}

	/// Returns an immutable reference to the underlying slice.
	pub fn as_slice(&self) -> &[T] {
		unsafe { self.slice.as_ref() }
	}

	/// Returns a mutable reference to the underlying slice.
	pub fn as_slice_mut(&mut self) -> &mut [T] {
		unsafe { self.slice.as_mut() }
	}

	/// Returns the allocation as pointer.
	pub fn as_ptr(&self) -> *const T {
		self.as_slice().as_ptr() as _
	}

	/// Returns the allocation as mutable pointer.
	pub fn as_ptr_mut(&mut self) -> *mut T {
		self.as_slice_mut().as_mut_ptr() as _
	}

	/// Returns the size of the allocation in number of elements.
	pub fn len(&self) -> usize {
		self.slice.len()
	}

	/// Changes the size of the memory allocation.
	///
	/// All new elements are uninitialized.
	///
	/// `n` is the new size of the chunk of memory (in number of elements).
	///
	/// If the reallocation fails, the chunk is left untouched and the function returns an error.
	///
	/// # Safety
	///
	/// Since the memory is not initialized, objects in the allocation might be
	/// in an inconsistent state.
	pub unsafe fn realloc(&mut self, n: NonZeroUsize) -> AllocResult<()> {
		let len = n
			.checked_mul(size_of::<T>().try_into().unwrap())
			.ok_or_else(|| AllocError)?;
		let ptr = realloc(self.slice.cast(), len)?;
		self.slice =
			NonNull::new(slice::from_raw_parts_mut::<T>(ptr.cast().as_mut(), n.get())).unwrap();

		Ok(())
	}

	/// Frees the allocation.
	pub fn free(self) {}
}

impl<T: Default> Alloc<T> {
	/// Allocates `size` element in the kernel memory and returns a structure
	/// wrapping a slice allowing to access it.
	///
	/// If the allocation fails, the function shall return an error.
	///
	/// The function will fill the memory with the default value for the object T.
	pub fn new_default(size: NonZeroUsize) -> AllocResult<Self> {
		let mut alloc = unsafe {
			// Safe because the memory is set right after
			Self::new(size)?
		};
		for i in alloc.as_slice_mut().iter_mut() {
			unsafe {
				// Safe because the pointer is in the range of the slice
				ptr::write(i, T::default());
			}
		}

		Ok(alloc)
	}

	/// Resizes the current allocation.
	///
	/// If new elements are added, the function initializes them with the default value.
	///
	/// `n` is the new size of the chunk of memory (in number of elements).
	///
	/// # Safety
	///
	/// If elements are removed, the function `drop` is **not** called on them.
	/// Thus, the caller must take care of dropping the elements first.
	pub unsafe fn realloc_default(&mut self, n: NonZeroUsize) -> AllocResult<()> {
		let curr_size = self.len();
		self.realloc(n)?;

		for i in curr_size..n.get() {
			unsafe {
				// Safe because the pointer is in the range of the slice
				ptr::write(&mut self.as_slice_mut()[i], T::default());
			}
		}

		Ok(())
	}
}

impl<T: Clone> Alloc<T> {
	/// Allocates `size` element in the kernel memory and returns a structure
	/// wrapping a slice allowing to access it.
	///
	/// If the allocation fails, the function shall return an error.
	///
	/// `val` is a value that will be cloned to fill the memory.
	pub fn new_clonable(size: NonZeroUsize, val: T) -> AllocResult<Self> {
		let mut alloc = unsafe {
			// Safe because the memory is set right after
			Self::new(size)?
		};
		for i in alloc.as_slice_mut().iter_mut() {
			unsafe {
				// Safe because the pointer is in the range of the slice
				ptr::write(i, val.clone());
			}
		}

		Ok(alloc)
	}
}

impl<T> Index<usize> for Alloc<T> {
	type Output = T;

	#[inline]
	fn index(&self, index: usize) -> &Self::Output {
		&self.as_slice()[index]
	}
}

impl<T> IndexMut<usize> for Alloc<T> {
	#[inline]
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.as_slice_mut()[index]
	}
}

impl<T> Drop for Alloc<T> {
	fn drop(&mut self) {
		unsafe {
			free(self.slice.cast());
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::memory;
	use crate::memory::buddy;
	use crate::util::math;
	use core::slice;

	#[test_case]
	fn alloc_free1() {
		let usage = buddy::allocated_pages_count();

		unsafe {
			let ptr = alloc(NonZeroUsize::new(1).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, 1).fill(!0);

			free(ptr);
		}

		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free1() {
		let usage = buddy::allocated_pages_count();

		unsafe {
			let ptr = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, 8).fill(!0);

			free(ptr);
		}

		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free2() {
		let usage = buddy::allocated_pages_count();

		unsafe {
			let ptr = alloc(NonZeroUsize::new(memory::PAGE_SIZE).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, memory::PAGE_SIZE).fill(!0);

			free(ptr);
		}

		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free3() {
		let usage = buddy::allocated_pages_count();

		unsafe {
			let ptr = alloc(NonZeroUsize::new(memory::PAGE_SIZE * 10).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, memory::PAGE_SIZE * 10).fill(!0);

			free(ptr);
		}

		assert_eq!(usage, buddy::allocated_pages_count());
	}

	#[test_case]
	fn alloc_free_fifo() {
		let usage = buddy::allocated_pages_count();

		unsafe {
			let mut ptrs: [NonNull<c_void>; 1024] = [NonNull::new(1 as _).unwrap(); 1024];

			for i in 0..ptrs.len() {
				let size = i + 1;
				let ptr = alloc(NonZeroUsize::new(size).unwrap()).unwrap();
				slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, size).fill(!0);

				ptrs[i] = ptr;
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
			slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, i).fill(!0);

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
				slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, i).fill(!0);
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
				slice::from_raw_parts_mut(ptr.as_ptr() as *mut u8, i).fill(!0);
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
			slice::from_raw_parts_mut(ptr0.as_ptr() as *mut u8, 8).fill(!0);
			let mut ptr1 = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr1.as_ptr() as *mut u8, 8).fill(!0);

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
			slice::from_raw_parts_mut(ptr0.as_ptr() as *mut u8, 8).fill(!0);
			let mut ptr1 = alloc(NonZeroUsize::new(8).unwrap()).unwrap();
			slice::from_raw_parts_mut(ptr1.as_ptr() as *mut u8, 8).fill(!0);

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
