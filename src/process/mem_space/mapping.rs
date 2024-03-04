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

//! A memory mapping is a region of virtual memory that a process can access.
//!
//! Mappings may be created at the process's creation or by the process itself using
//! system calls.

use super::gap::MemGap;
use crate::{
	errno::AllocError,
	file::vfs,
	memory,
	memory::{
		vmem,
		vmem::{VMem, VMemTransaction},
	},
	process::{
		mem_space::{
			residence::{MapResidence, Page, ResidencePage},
			COPY_BUFFER,
		},
		AllocResult, EResult,
	},
	util::{collections::vec::Vec, io::IO, ptr::arc::Arc, TryClone},
};
use core::{ffi::c_void, fmt, num::NonZeroUsize, ops::Range, slice};

/// A mapping in a memory space.
pub struct MemMapping {
	/// Address on the virtual memory to the beginning of the mapping
	begin: *mut c_void,
	/// The size of the mapping in pages.
	size: NonZeroUsize,
	/// The mapping's flags.
	flags: u8,
	/// The residence of the mapping.
	residence: MapResidence,

	/// The list of allocated physical pages. Each page may be shared with other mappings.
	phys_pages: Vec<Option<Arc<ResidencePage>>>,
}

impl MemMapping {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `begin` is the pointer on the virtual memory to the beginning of the
	/// mapping. This pointer must be page-aligned.
	/// - `size` is the size of the mapping in pages. The size must be greater
	/// than 0.
	/// - `flags` the mapping's flags.
	/// - `file` is the open file the mapping points to, with an offset in it. If `None`, the
	///   mapping doesn't point to any file.
	/// - `residence` is the residence for the mapping.
	pub fn new(
		begin: *mut c_void,
		size: NonZeroUsize,
		flags: u8,
		residence: MapResidence,
	) -> AllocResult<Self> {
		debug_assert!(begin.is_aligned_to(memory::PAGE_SIZE));
		let mut phys_pages = Vec::new();
		phys_pages.resize(size.get(), None)?;
		Ok(Self {
			begin,
			size,
			flags,
			residence,

			phys_pages,
		})
	}

	/// Returns a pointer on the virtual memory to the beginning of the mapping.
	pub fn get_begin(&self) -> *mut c_void {
		self.begin
	}

	/// Returns the size of the mapping in memory pages.
	pub fn get_size(&self) -> NonZeroUsize {
		self.size
	}

	/// Returns the mapping's flags.
	pub fn get_flags(&self) -> u8 {
		self.flags
	}

	/// Tells whether the page at offset `offset` is waiting for Copy-On-Write.
	fn is_cow(&self, offset: usize) -> bool {
		if self.flags & super::MAPPING_FLAG_SHARED != 0 {
			return false;
		}
		// Check if currently shared
		self.phys_pages[offset]
			.as_ref()
			.map(|a| Arc::strong_count(a) > 1)
			.unwrap_or(false)
	}

	/// Returns the flags for the virtual memory context for the given virtual page offset.
	///
	/// Arguments:
	/// - `allocated` tells whether the page has been physically allocated.
	/// - `offset` is the offset of the page in the mapping.
	fn get_vmem_flags(&self, allocated: bool, offset: usize) -> u32 {
		let mut flags = 0;
		if self.flags & super::MAPPING_FLAG_WRITE != 0 && allocated && !self.is_cow(offset) {
			#[cfg(target_arch = "x86")]
			flags |= vmem::x86::FLAG_WRITE;
		}
		if self.flags & super::MAPPING_FLAG_USER != 0 {
			#[cfg(target_arch = "x86")]
			flags |= vmem::x86::FLAG_USER;
		}
		flags
	}

	/// Forces allocation of a physical page for the given offset in the mapping.
	///
	/// The function also applies the mapping of the page to the given `vmem_transaction`.
	pub(super) fn alloc(
		&mut self,
		offset: usize,
		vmem_transaction: &mut VMemTransaction<false>,
	) -> AllocResult<()> {
		// Allocate new page
		let next = self.residence.acquire_page(offset)?;
		let next_physaddr = unsafe { (*next).as_ptr() };
		// Replace the previous page, which is kept until the function returns to prevent dropping
		// before copy
		let page = self
			.phys_pages
			// Bound check
			.get_mut(offset)
			.ok_or(AllocError)?;
		let prev = page.replace(next);
		// Tells whether a copy is necessary
		let copy = prev.is_some();
		// Map previous page for copy
		if let Some(prev) = &prev {
			let physaddr = unsafe { (**prev).as_ptr() as _ };
			vmem_transaction.map(physaddr, COPY_BUFFER as _, 0)?;
		}
		// Map next page for copy or zeroing
		let next_virtaddr = (self.begin as usize + offset * memory::PAGE_SIZE) as _;
		let flags = self.get_vmem_flags(true, offset);
		vmem_transaction.map(next_physaddr as _, next_virtaddr, flags)?;
		// Copy or zero page only for normal residences
		if !self.residence.is_normal() {
			return Ok(());
		}
		unsafe {
			let dest = &mut *(next_virtaddr as *mut Page);
			// Switching to make sure the right vmem is bound, but this should already be the case
			// so consider this has no cost
			vmem::switch(vmem_transaction.vmem, || {
				vmem::write_lock_wrap(|| {
					if copy {
						// Copy
						let src = &mut *(COPY_BUFFER as *mut Page);
						dest.copy_from_slice(src);
					} else {
						// Zero page
						dest.fill(0);
					}
				});
			});
		}
		Ok(())
	}

	/// Applies the mapping to the given `vmem_transaction`.
	pub fn apply_to(&mut self, vmem_transaction: &mut VMemTransaction<false>) -> AllocResult<()> {
		let default_page = self.residence.get_default_page();
		if let Some(default_page) = default_page {
			for (i, phys_page) in self.phys_pages.iter().enumerate() {
				let physaddr = phys_page.as_ref().map(|p| unsafe { (**p).as_ptr() });
				let allocated = physaddr.is_some();
				let physaddr = physaddr.unwrap_or(default_page.as_ptr() as _);
				let virtaddr = (self.begin as usize + i * memory::PAGE_SIZE) as _;
				let flags = self.get_vmem_flags(allocated, i);
				vmem_transaction.map(physaddr as _, virtaddr, flags)?;
			}
		} else {
			for i in 0..self.size.get() {
				self.alloc(i, vmem_transaction)?;
			}
		}
		Ok(())
	}

	/// Splits the current mapping, creating up to two new mappings and one gap.
	///
	/// Arguments:
	/// - `begin` is the index of the first page to be unmapped.
	/// - `size` is the number of pages to unmap.
	///
	/// If the region to be unmapped is out of bounds, it is truncated to the end of the mapping.
	///
	/// The newly created mappings correspond to the remaining pages.
	///
	/// The newly created gap corresponds to the unmapped portion.
	///
	/// If the mapping is completely unmapped, the function returns no new mappings.
	pub fn split(
		&self,
		begin: usize,
		size: usize,
	) -> AllocResult<(Option<Self>, Option<MemGap>, Option<Self>)> {
		let begin_ptr = (self.begin as usize + begin * memory::PAGE_SIZE) as _;
		let prev = NonZeroUsize::new(begin)
			.map(|size| {
				Ok(MemMapping {
					begin: self.begin,
					size,
					flags: self.flags,
					residence: self.residence.clone(),

					phys_pages: Vec::from_slice(&self.phys_pages[..size.get()])?,
				})
			})
			.transpose()?;
		let gap = NonZeroUsize::new(size).map(|size| MemGap {
			begin: begin_ptr,
			size,
		});
		// The gap's end
		let end = begin + size;
		let next = self
			.size
			.get()
			.checked_sub(end)
			.and_then(NonZeroUsize::new)
			.map(|size| {
				let begin = (self.begin as usize + end * memory::PAGE_SIZE) as _;
				let mut residence = self.residence.clone();
				residence.offset_add(end);
				Ok(Self {
					begin,
					size,
					flags: self.flags,
					residence,

					phys_pages: Vec::from_slice(&self.phys_pages[end..])?,
				})
			})
			.transpose()?;
		Ok((prev, gap, next))
	}

	/// Synchronizes the data on the memory mapping back to the filesystem.
	///
	/// `vmem` is the virtual memory context to read from.
	///
	/// The function does nothing if:
	/// - The mapping is not shared
	/// - The mapping is not associated with a file
	/// - The associated file has been removed or cannot be accessed
	///
	/// If the mapping is lock, the function returns [`crate::errno::EBUSY`].
	pub fn fs_sync(&self, vmem: &VMem) -> EResult<()> {
		if self.flags & super::MAPPING_FLAG_SHARED == 0 {
			return Ok(());
		}
		// TODO if locked, EBUSY
		// Get file
		let MapResidence::File {
			location,
			off,
		} = &self.residence
		else {
			return Ok(());
		};
		let Ok(file_mutex) = vfs::get_file_from_location(location) else {
			return Ok(());
		};
		// Sync
		unsafe {
			vmem::switch(vmem, || {
				// TODO Make use of dirty flag if present on the current architecture to update
				// only pages that have been modified
				let slice = slice::from_raw_parts(
					self.begin as *mut u8,
					self.size.get() * memory::PAGE_SIZE,
				);
				let mut file = file_mutex.lock();
				let mut i = 0;
				while i < slice.len() {
					let l = file.write(*off, &slice[i..])?;
					i += l as usize;
				}
				Ok(())
			})
		}
	}

	/// Unmaps the mapping using the given `vmem_transaction`.
	///
	/// `range` is the range of pages affect by the unmap. Pages outside of this range are left
	/// untouched.
	///
	/// If applicable, the function synchronizes the data on the pages to be unmapped to the disk.
	///
	/// This function doesn't flush the virtual memory context.
	///
	/// On success, the function returns the transaction.
	pub fn unmap(
		&self,
		pages_range: Range<usize>,
		vmem_transaction: &mut VMemTransaction<false>,
	) -> EResult<()> {
		self.fs_sync(vmem_transaction.vmem)?;
		let begin = (self.begin as usize + pages_range.start * memory::PAGE_SIZE) as _;
		let len = pages_range.end - pages_range.start;
		vmem_transaction.unmap_range(begin, len)?;
		Ok(())
	}
}

impl TryClone for MemMapping {
	fn try_clone(&self) -> AllocResult<Self> {
		Ok(Self {
			begin: self.begin,
			size: self.size,
			flags: self.flags,
			residence: self.residence.clone(),

			phys_pages: self.phys_pages.try_clone()?,
		})
	}
}

impl fmt::Debug for MemMapping {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let end = (self.begin as usize + self.size.get() * memory::PAGE_SIZE) as *const c_void;
		write!(
			f,
			"MemMapping {{ begin: {:p}, end: {:p}, flags: {} }}",
			self.begin, end, self.flags,
		)
	}
}
