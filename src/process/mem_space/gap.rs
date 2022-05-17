//! A gap is a region of the virtual memory which is available for allocation.

use core::cmp::min;
use core::ffi::c_void;
use core::fmt;
use crate::memory;
use crate::util::FailableClone;
use crate::util;

/// A gap in the memory space that can use for new mappings.
#[derive(Clone, Debug)]
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

	/// Returns a pointer on the virtual memory to the end of the gap.
	pub fn get_end(&self) -> *const c_void {
		unsafe {
			self.begin.add(self.size * memory::PAGE_SIZE)
		}
	}

	/// Returns the size of the gap in memory pages.
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Creates new gaps to replace the current one after mapping memory onto it. After calling
	/// this function, the callee shall removed the current gap from its container and insert the
	/// new ones in it.
	/// `off` is the offset of the part to consume.
	/// `size` is the size of the part to consume.
	/// The function returns a new gap. If the gap is fully consumed, the function returns None.
	pub fn consume(&self, off: usize, size: usize) -> (Option<Self>, Option<Self>) {
		// The new gap located before the mapping
		let mut left = None;
		if off > 0 {
			let addr = self.begin;
			let size = min(off, self.size);

			if size > 0 {
				left = Some(Self::new(addr, size));
			}
		}

		// The new gap located after the mapping
		let mut right = None;
		if off + size < self.size {
			let addr = ((self.begin as usize) + ((off + size) * memory::PAGE_SIZE)) as _;
			let size = self.size - (off + size);

			if size > 0 {
				right = Some(Self::new(addr, size));
			}
		}

		(left, right)
	}

	/// Merges the given gap `gap` with the current gap. If the gaps are not adjacent, the function
	/// does nothing.
	pub fn merge(&mut self, gap: Self) {
		// If `gap` is before
		if self.get_begin() == gap.get_end() {
			self.begin = gap.begin;
			self.size += gap.size;
		}

		// If `gap` is after
		if self.get_end() == gap.get_begin() {
			self.size += gap.size;
		}
	}
}

crate::failable_clone_impl!(MemGap);

impl fmt::Display for MemGap {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "begin: {:p}; end: {:p}", self.begin, self.get_end())
	}
}
