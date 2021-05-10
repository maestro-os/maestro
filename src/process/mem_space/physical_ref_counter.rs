//! This module implements a physical pages reference counter, which is used to keep track of the
//! physical pages that are referenced by several mappings.
//! For each page that is referenced more than once, the counter stores the number of references to
//! that page. A binary tree is used to find the page from its pointer.

use core::cmp::Ordering;
use core::ffi::c_void;
use crate::errno::Errno;
use crate::memory;
use crate::util::container::binary_tree::BinaryTree;
use crate::util;

/// The threshold under which the structure stops counting the number of references for a page.
const COUNT_THRESHOLD: usize = 1;

/// Structure representing a reference counter for a given page.
pub struct PageRefCounter {
	/// Pointer to the physical page associated with the counter.
	page_addr: *const c_void,
	/// The number of references to the page.
	references: usize,
}

impl Ord for PageRefCounter {
	fn cmp(&self, other: &Self) -> Ordering {
		self.page_addr.cmp(&other.page_addr)
	}
}

impl Eq for PageRefCounter {}

impl PartialEq for PageRefCounter {
	fn eq(&self, other: &Self) -> bool {
		self.page_addr == other.page_addr
	}
}

impl PartialOrd for PageRefCounter {
	fn partial_cmp(&self, other: &Self) -> Option::<Ordering> {
		Some(self.page_addr.cmp(&other.page_addr))
	}
}

impl PartialEq::<*const c_void> for PageRefCounter {
	fn eq(&self, other: &*const c_void) -> bool {
		self.page_addr == *other
	}
}

impl PartialOrd::<*const c_void> for PageRefCounter {
	fn partial_cmp(&self, other: &*const c_void) -> Option::<Ordering> {
		Some(self.page_addr.cmp(other))
	}
}

/// Structure representing the reference counter for all physical pages.
pub struct PhysRefCounter {
	/// The binary tree storing the number of references for each pages.
	tree: BinaryTree::<PageRefCounter>,
}

impl PhysRefCounter {
	/// Creates a new instance.
	pub const fn new() -> Self {
		Self {
			tree: BinaryTree::<PageRefCounter>::new(),
		}
	}

	/// Returns the number of references for the given page.
	/// `ptr` is the physical address of the page.
	/// If the page isn't stored into the structure, the function returns `0`.
	pub fn get_ref_count(&self, ptr: *const c_void) -> usize {
		let ptr = util::down_align(ptr, memory::PAGE_SIZE);
		if let Some(counter) = self.tree.get(ptr) {
			counter.references
		} else {
			0
		}
	}

	/// Tells whether the given page is shared.
	/// `ptr` is the physical address of the page.
	pub fn is_shared(&self, ptr: *const c_void) -> bool {
		self.get_ref_count(ptr) > COUNT_THRESHOLD
	}

	/// Increments the references count for the given page. If the page isn't stored into the
	/// structure, the function adds it.
	/// `ptr` is the physical address of the page.
	pub fn increment(&mut self, ptr: *const c_void) -> Result<(), Errno> {
		let ptr = util::down_align(ptr, memory::PAGE_SIZE);
		if let Some(counter) = self.tree.get_mut(ptr) {
			counter.references += COUNT_THRESHOLD;
			Ok(())
		} else {
			self.tree.insert(PageRefCounter {
				page_addr: ptr,
				references: COUNT_THRESHOLD + 1,
			})
		}
	}

	/// Decrements the references count for the given page. If the page's counter reaches 1, the
	/// function removes the page from the structure.
	/// `ptr` is the physical address of the page.
	pub fn decrement(&mut self, ptr: *const c_void) {
		let ptr = util::down_align(ptr, memory::PAGE_SIZE);
		if let Some(counter) = self.tree.get_mut(ptr) {
			counter.references -= 1;
			if counter.references <= COUNT_THRESHOLD {
				self.tree.remove(ptr);
			}
		}
	}
}
