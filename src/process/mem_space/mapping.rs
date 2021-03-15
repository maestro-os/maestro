/// TODO doc

use core::cmp::Ordering;
use core::ffi::c_void;
use crate::memory;
use crate::util;

/// A mapping in the memory space.
pub struct MemMapping {
	/// Pointer on the virtual memory to the beginning of the mapping
	begin: *const c_void,
	/// The size of the mapping in pages.
	size: usize,
	/// The mapping's flags.
	flags: u8,

	// TODO Add sharing informations
}

impl Ord for MemMapping {
	fn cmp(&self, other: &Self) -> Ordering {
		self.begin.cmp(&other.begin)
	}
}

impl Eq for MemMapping {}

impl PartialEq for MemMapping {
	fn eq(&self, other: &Self) -> bool {
		self.begin == other.begin
	}
}

impl PartialOrd for MemMapping {
	fn partial_cmp(&self, other: &Self) -> Option::<Ordering> {
		Some(self.begin.cmp(&other.begin))
	}
}

impl PartialEq::<*const c_void> for MemMapping {
	fn eq(&self, other: &*const c_void) -> bool {
		self.begin == *other
	}
}

impl PartialOrd::<*const c_void> for MemMapping {
	fn partial_cmp(&self, other: &*const c_void) -> Option::<Ordering> {
		Some(self.begin.cmp(other))
	}
}

impl MemMapping {
	/// Creates a new instance.
	/// `begin` is the pointer on the virtual memory to the beginning of the mapping. This pointer
	/// must be page-aligned.
	/// `size` is the size of the mapping in pages. The size must be greater than 0.
	/// `flags` the mapping's flags
	pub fn new(begin: *const c_void, size: usize, flags: u8) -> Self {
		debug_assert!(util::is_aligned(begin, memory::PAGE_SIZE));
		debug_assert!(size > 0);

		Self {
			begin: begin,
			size: size,
			flags: flags,
		}
	}

	/// Returns a pointer on the virtual memory to the beginning of the mapping.
	pub fn get_begin(&self) -> *const c_void {
		self.begin
	}

	/// Returns the size of the mapping in memory pages.
	pub fn get_size(&self) -> usize {
		self.size
	}

	// TODO
}
