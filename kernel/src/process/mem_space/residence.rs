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

//! A map residence provides information about how to populate a memory mapping.

use crate::{
	file::{open_file::OpenFile, FileLocation},
	memory,
	memory::buddy,
};
use core::{alloc::AllocError, ptr::NonNull};
use utils::{collections::vec::Vec, errno::AllocResult, lock::Mutex, ptr::arc::Arc};

/// Type representing a memory page.
pub type Page = [u8; memory::PAGE_SIZE];

/// Returns a physical address to the default zeroed page.
///
/// This page is meant to be mapped in read-only and is a placeholder for pages that are
/// accessed without being allocated nor written.
#[inline]
fn zeroed_page() -> NonNull<Page> {
	#[repr(align(4096))]
	struct DefaultPage(Page);
	static DEFAULT_PAGE: DefaultPage = DefaultPage([0; memory::PAGE_SIZE]);
	let ptr = memory::kern_to_phys(DEFAULT_PAGE.0.as_ptr() as _) as *mut _;
	NonNull::new(ptr).unwrap()
}

/// Wrapper for an allocated physical page of memory.
///
/// On drop, the page is freed.
#[derive(Debug)]
pub struct ResidencePage(NonNull<Page>);

impl ResidencePage {
	/// Creates a new instance from the given page.
	///
	/// **Note**: The resulting `ResidencePage` takes the ownership of the page.
	pub fn new(page: NonNull<Page>) -> Self {
		Self(page)
	}

	/// Returns the page's pointer.
	///
	/// # Safety
	///
	/// Using the pointed to by the given pointer is undefined.
	pub unsafe fn ptr(&self) -> *const Page {
		self.0.as_ptr()
	}
}

impl Drop for ResidencePage {
	fn drop(&mut self) {
		unsafe {
			buddy::free(self.0.as_ptr() as _, 0);
		}
	}
}

// TODO when reaching the last reference to the open file, close it on unmap
// TODO Disallow clone and use a special function + Drop to increment/decrement reference counters
/// A map residence is the source of the data on a physical page used by a mapping. It is also the
/// location to which the data is to be synchronized when modified.
#[derive(Clone, Debug)]
pub enum MapResidence {
	/// The mapping does not reside anywhere except on the main memory.
	Normal,
	/// The mapping points to a static location, which may or may not be shared between several
	/// memory spaces.
	Static {
		/// The list of memory pages, in order.
		///
		/// The outer [`Arc`] is here to allow cloning [`MapResidence`] without a memory
		/// allocation. The inner [`Arc`] is here to conveniently match with the return type of
		/// [`Self::acquire_page`].
		pages: Arc<Vec<Arc<ResidencePage>>>,
	},
	/// The mapping resides in a file.
	File {
		/// The location of the file.
		location: FileLocation,
		/// The offset of the mapping in the file.
		off: u64,
	},
	/// The mapping resides in swap memory.
	Swap {
		/// The location of the swap memory.
		swap_file: Arc<Mutex<OpenFile>>,
		/// The ID of the slot occupied by the mapping.
		slot_id: u32,
		/// The page offset in the slot.
		page_off: usize,
	},
}

impl MapResidence {
	/// Tells whether the residence is normal.
	pub fn is_normal(&self) -> bool {
		matches!(self, MapResidence::Normal)
	}

	/// Returns the default physical page for the mapping, if applicable.
	///
	/// If no default page exist, pages should be allocated directly.
	pub fn get_default_page(&self) -> Option<NonNull<Page>> {
		match self {
			MapResidence::Normal => Some(zeroed_page()),
			_ => None,
		}
	}

	/// Adds a value of `pages` pages to the offset of the residence, if applicable.
	pub fn offset_add(&mut self, pages: usize) {
		match self {
			Self::File {
				off, ..
			} => *off += pages as u64 * memory::PAGE_SIZE as u64,
			Self::Swap {
				page_off, ..
			} => *page_off += pages,
			_ => {}
		}
	}

	/// Returns an allocated page of memory for the given `offset`.
	///
	/// If no page already exist for this offset, the function allocates one. Else, it reuses the
	/// one that is already allocated.
	///
	/// The returned page is already populated with the necessary data. It is released when
	/// [`ResidencePage`] is dropped.
	pub fn acquire_page(&self, offset: usize) -> AllocResult<Arc<ResidencePage>> {
		match self {
			MapResidence::Normal => {
				let page = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?.cast();
				Arc::new(ResidencePage::new(page))
			}
			MapResidence::Static {
				pages,
			} => pages.get(offset).cloned().ok_or(AllocError),
			MapResidence::File {
				location: _,
				off: _,
			} => {
				// TODO get physical page for this offset
				todo!();
			}
			MapResidence::Swap {
				..
			} => {
				// TODO
				todo!();
			}
		}
	}
}
