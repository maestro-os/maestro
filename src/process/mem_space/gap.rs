//! A gap is a region of the virtual memory which is available for allocation.

use core::cmp::Ordering;
use core::ffi::c_void;
use crate::memory;
use crate::util::FailableClone;
use crate::util::list::ListNode;
use crate::util;

/// A gap in the memory space that can use for new mappings.
pub struct MemGap {
	/// Pointer on the virtual memory to the beginning of the gap
	begin: *const c_void,
	/// The size of the gap in pages.
	size: usize,

	/// The node in the list storing the gap to be searched by size.
	pub list: ListNode,
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

			list: ListNode::new_single(),
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

impl Ord for MemGap {
	fn cmp(&self, other: &Self) -> Ordering {
		self.begin.cmp(&other.begin)
	}
}

impl Eq for MemGap {}

impl PartialEq for MemGap {
	fn eq(&self, other: &Self) -> bool {
		self.begin == other.begin
	}
}

impl PartialOrd for MemGap {
	fn partial_cmp(&self, other: &Self) -> Option::<Ordering> {
		Some(self.begin.cmp(&other.begin))
	}
}

impl PartialEq::<*const c_void> for MemGap {
	fn eq(&self, other: &*const c_void) -> bool {
		self.begin == *other
	}
}

impl PartialOrd::<*const c_void> for MemGap {
	fn partial_cmp(&self, other: &*const c_void) -> Option::<Ordering> {
		Some(self.begin.cmp(other))
	}
}

impl Clone for MemGap {
	fn clone(&self) -> Self {
		Self {
			begin: self.begin,
			size: self.size,

			list: ListNode::new_single(),
		}
	}
}

crate::failable_clone_impl!(MemGap);
