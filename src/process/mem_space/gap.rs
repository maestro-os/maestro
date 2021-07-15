//! A gap is a region of the virtual memory which is available for allocation.

use core::ffi::c_void;
use crate::memory;
use crate::util::FailableClone;
use crate::util;

/// A gap in the memory space that can use for new mappings.
pub struct MemGap {
	/// Pointer on the virtual memory to the beginning of the gap
	begin: *const c_void,
	/// The size of the gap in pages.
	size: usize,
}

impl MemGap {
	/// Creates a new instance.
	/// `begin` is a pointer on the virtual memory to the beginning of the gap. This pointer must
	/// be page-aligned.
	/// `size` is the size of the gap in pages. The size must be greater than 0.
	pub fn new(begin: *const c_void, size: usize) -> Self {
		debug_assert!(util::is_aligned(begin, memory::PAGE_SIZE));
		debug_assert!(size > 0);

		Self {
			begin,
			size,
		}
	}

	/// Returns a pointer on the virtual memory to the beginning of the gap.
	pub fn get_begin(&self) -> *const c_void {
		self.begin
	}

	/// Returns the size of the gap in memory pages.
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Creates a new gap to replace the current one after mapping memory on it. After calling
	/// this function, the callee shall removed the current gap from its container before inserting
	/// the new one in it.
	/// `size` is the size of the part that has been consumed on the gap.
	/// The function returns a new gap. If the gap is fully consumed, the function returns None.
	pub fn consume(&self, size: usize) -> Option::<Self> {
		debug_assert!(size <= self.size);
		if size < self.size {
			let new_addr = ((self.begin as usize) + (size * memory::PAGE_SIZE)) as _;
			let new_size = self.size - size;
			Some(Self::new(new_addr, new_size))
		} else {
			None
		}
	}
}

impl Clone for MemGap {
	fn clone(&self) -> Self {
		Self {
			begin: self.begin,
			size: self.size,
		}
	}
}

crate::failable_clone_impl!(MemGap);
