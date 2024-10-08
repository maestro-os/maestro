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
	file::File,
	memory::{buddy, PhysAddr, VirtAddr},
};
use core::alloc::AllocError;
use utils::{collections::vec::Vec, errno::AllocResult, limits::PAGE_SIZE, ptr::arc::Arc};

/// Type representing a memory page.
pub type Page = [u8; PAGE_SIZE];

/// Returns a physical address to the default zeroed page.
///
/// This page is meant to be mapped in read-only and is a placeholder for pages that are
/// accessed without being allocated nor written.
#[inline]
fn zeroed_page() -> PhysAddr {
	#[repr(align(4096))]
	struct DefaultPage(Page);
	static DEFAULT_PAGE: DefaultPage = DefaultPage([0; PAGE_SIZE]);
	VirtAddr::from(DEFAULT_PAGE.0.as_ptr())
		.kernel_to_physical()
		.unwrap()
}

/// Wrapper for an allocated physical page of memory.
///
/// On drop, the page is freed.
#[derive(Debug)]
pub struct ResidencePage(PhysAddr);

impl ResidencePage {
	/// Creates a new instance from the given physical address, taking ownership over it.
	pub fn new(page: PhysAddr) -> Self {
		Self(page)
	}

	/// Returns the page's physical address.
	pub fn get(&self) -> PhysAddr {
		self.0
	}
}

impl Drop for ResidencePage {
	fn drop(&mut self) {
		unsafe {
			buddy::free(self.0, 0);
		}
	}
}

// TODO when reaching the last reference to the open file, close it on unmap
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
		/// The mapped file.
		file: Arc<File>,
		/// The offset of the mapping in the file.
		off: u64,
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
	pub fn get_default_page(&self) -> Option<PhysAddr> {
		match self {
			MapResidence::Normal => Some(zeroed_page()),
			_ => None,
		}
	}

	/// Adds a value of `pages` pages to the offset of the residence, if applicable.
	pub fn offset_add(&mut self, pages: usize) {
		if let Self::File {
			off, ..
		} = self
		{
			*off += pages as u64 * PAGE_SIZE as u64
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
				let page = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?;
				Arc::new(ResidencePage::new(page))
			}
			MapResidence::Static {
				pages,
			} => pages.get(offset).cloned().ok_or(AllocError),
			MapResidence::File {
				file: _,
				off: _,
			} => {
				// TODO get physical page for this offset
				todo!();
			}
		}
	}
}
