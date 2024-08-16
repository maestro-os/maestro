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

//! A file mapping is a view of a file in memory, which can be modified, shared between processes,
//! etc...

use crate::{memory, memory::buddy};
use core::ptr::NonNull;
use utils::{collections::hashmap::HashMap, errno::EResult};

/// Structure representing a mapped page for a file.
struct Page {
	/// The pointer to the page.
	ptr: NonNull<[u8; memory::PAGE_SIZE]>,
	/// The number of references to the page.
	ref_count: u32,
}

/// A file mapped partially or totally into memory.
#[derive(Default)]
struct MappedFile {
	/// The list of mappings, ordered by offset in pages.
	pages: HashMap<usize, Page>,
}

impl MappedFile {
	/// Acquires the page at the given offset, incrementing the number of referencces to it.
	///
	/// If the page is not mapped, the function maps it.
	///
	/// `off` is the offset of the page in pages count.
	pub fn acquire_page(&mut self, off: usize) -> EResult<&mut Page> {
		if !self.pages.contains_key(&off) {
			self.pages.insert(
				off,
				Page {
					ptr: buddy::alloc_kernel(0)?.cast(),
					ref_count: 1,
				},
			)?;
		}

		let page = self.pages.get_mut(&off).unwrap();
		page.ref_count += 1;

		Ok(page)
	}

	/// Releases the page at the given offset, decrementing the number of references to it.
	///
	/// If the references count reaches zero, the function synchonizes the page to the disk and
	/// unmaps it.
	///
	/// `off` is the offset of the page in pages count.
	///
	/// If the page is not mapped, the function does nothing.
	pub fn release_page(&mut self, off: usize) {
		let Some(page) = self.pages.get_mut(&off) else {
			return;
		};

		page.ref_count -= 1;
		if page.ref_count == 0 {
			self.pages.remove(&off);
		}
	}
}
