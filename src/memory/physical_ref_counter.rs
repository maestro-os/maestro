//! This module implements a physical pages reference counter, which is used to
//! keep track of the physical pages that are referenced by several mappings.
//!
//! For each page that is referenced more than once, the counter stores the
//! number of references to that page.

use crate::{errno::AllocResult, memory, util, util::container::hashmap::HashMap};
use core::ffi::c_void;

/// Structure representing the reference counter for all physical pages.
pub struct PhysRefCounter {
	/// The the number of references for each pages.
	ref_counts: HashMap<*const c_void, usize>,
}

impl PhysRefCounter {
	/// Creates a new instance.
	pub const fn new() -> Self {
		Self {
			ref_counts: HashMap::new(),
		}
	}

	/// Returns the number of references for the given page.
	///
	/// `ptr` is the physical address of the page.
	///
	/// If the page isn't stored into the structure, the function returns `0`.
	pub fn get_ref_count(&self, ptr: *const c_void) -> usize {
		let ptr = util::down_align(ptr, memory::PAGE_SIZE);
		self.ref_counts.get(&ptr).cloned().unwrap_or(0)
	}

	/// Tells whether the given page is shared.
	///
	/// `ptr` is the physical address of the page.
	pub fn is_shared(&self, ptr: *const c_void) -> bool {
		self.get_ref_count(ptr) > 1
	}

	/// Tells whether the given page can be freed.
	///
	/// `ptr` is the physical address of the page.
	pub fn can_free(&self, ptr: *const c_void) -> bool {
		self.get_ref_count(ptr) < 1
	}

	/// Increments the references count for the given page.
	///
	/// `ptr` is the physical address of the page.
	///
	/// If the page isn't stored into the structure, the function adds it.
	pub fn increment(&mut self, ptr: *const c_void) -> AllocResult<()> {
		let ptr = util::down_align(ptr, memory::PAGE_SIZE);

		if let Some(ref_count) = self.ref_counts.get_mut(&ptr) {
			*ref_count += 1;
		} else {
			self.ref_counts.insert(ptr, 1)?;
		}

		Ok(())
	}

	/// Decrements the references count for the given page.
	///
	/// `ptr` is the physical address of the page.
	///
	/// If the page's counter reaches 1, the function removes the page from the structure.
	pub fn decrement(&mut self, ptr: *const c_void) {
		let ptr = util::down_align(ptr, memory::PAGE_SIZE);

		if let Some(ref_count) = self.ref_counts.get_mut(&ptr) {
			*ref_count -= 1;

			if *ref_count < 1 {
				self.ref_counts.remove(&ptr);
			}
		}
	}
}
