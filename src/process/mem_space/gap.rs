//! A gap is a region of the virtual memory which is available for allocation.

use crate::memory;
use core::{cmp::min, ffi::c_void, fmt, num::NonZeroUsize};

/// A gap in the memory space that can use for new mappings.
#[derive(Clone)]
pub struct MemGap {
	/// Address on the virtual memory to the beginning of the gap
	pub(super) begin: *mut c_void,
	/// The size of the gap in pages.
	pub(super) size: NonZeroUsize,
}

impl MemGap {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `begin` is a pointer on the virtual memory to the beginning of the gap.
	/// This pointer must be page-aligned.
	/// - `size` is the size of the gap in pages.
	pub fn new(begin: *mut c_void, size: NonZeroUsize) -> Self {
		debug_assert!(begin.is_aligned_to(memory::PAGE_SIZE));
		Self {
			begin,
			size,
		}
	}

	/// Returns a pointer on the virtual memory to the beginning of the gap.
	#[inline]
	pub fn get_begin(&self) -> *mut c_void {
		self.begin
	}

	/// Returns a pointer on the virtual memory to the end of the gap.
	#[inline]
	pub fn get_end(&self) -> *mut c_void {
		unsafe { self.begin.add(self.size.get() * memory::PAGE_SIZE) }
	}

	/// Returns the size of the gap in memory pages.
	#[inline]
	pub fn get_size(&self) -> NonZeroUsize {
		self.size
	}

	/// Returns the offset in pages to the given address in the gap.
	#[inline]
	pub fn get_page_offset_for(&self, addr: *const c_void) -> usize {
		(addr as usize - self.begin as usize) / memory::PAGE_SIZE
	}

	/// Creates new gaps to replace the current one after mapping memory onto
	/// it.
	///
	/// Arguments:
	/// - `off` is the offset of the part to consume
	/// - `size` is the size of the part to consume
	///
	/// The function returns a new gap. If the gap is fully consumed, the
	/// function returns `(None, None)`.
	pub fn consume(&self, off: usize, size: usize) -> (Option<Self>, Option<Self>) {
		// The new gap located before the mapping
		let left = NonZeroUsize::new(off).map(|off| Self {
			begin: self.begin,
			size: min(off, self.size),
		});
		// The new gap located after the mapping
		let right = self
			.size
			.get()
			.checked_sub(off + size)
			.and_then(NonZeroUsize::new)
			.map(|gap_size| Self {
				begin: unsafe { self.begin.add((off + size) * memory::PAGE_SIZE) },
				size: gap_size,
			});
		(left, right)
	}

	/// Merges the given gap `other` with the current gap.
	///
	/// If the gaps are not adjacent, the function does nothing.
	pub fn merge(&mut self, other: &Self) {
		if self.begin == other.get_end() {
			// If `other` is before
			self.begin = other.begin;
			self.size = self.size.saturating_add(other.size.get());
		} else if self.get_end() == other.begin {
			// If `other` is after
			self.size = self.size.saturating_add(other.size.get());
		}
	}
}

impl fmt::Debug for MemGap {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			"MemGap {{ begin: {:p}, end: {:p} }}",
			self.begin,
			self.get_end()
		)
	}
}
