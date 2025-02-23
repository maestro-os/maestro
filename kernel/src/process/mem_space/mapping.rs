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
	arch::x86::paging,
	file::File,
	memory::{
		buddy,
		buddy::{Flags, ZONE_USER},
		vmem,
		vmem::{VMem, VMemTransaction},
		PhysAddr, VirtAddr,
	},
	process::mem_space::{
		zeroed_page, Page, COPY_BUFFER, MAP_ANONYMOUS, MAP_SHARED, PROT_EXEC, PROT_WRITE,
	},
};
use core::{alloc::AllocError, num::NonZeroUsize, ops::Range};
use utils::{
	collections::vec::Vec,
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
	TryClone,
};

/// Wrapper for an allocated physical page of memory.
///
/// On drop, the page is freed.
#[derive(Debug)]
pub struct AnonPage(PhysAddr);

impl AnonPage {
	/// Allocates a new, *uninitialized* page.
	pub fn new(flags: Flags) -> AllocResult<Self> {
		buddy::alloc(0, flags).map(Self)
	}

	/// Returns the page's physical address.
	pub fn get(&self) -> PhysAddr {
		self.0
	}
}

impl Drop for AnonPage {
	fn drop(&mut self) {
		unsafe {
			buddy::free(self.0, 0);
		}
	}
}

/// A mapping in a memory space.
#[derive(Debug)]
pub struct MemMapping {
	/// Address on the virtual memory to the beginning of the mapping
	addr: *mut u8,
	/// The size of the mapping in pages
	size: NonZeroUsize,
	/// Memory protection
	prot: u8,
	/// Mapping flags
	flags: u8,

	/// The mapped file, if any
	file: Option<Arc<File>>,
	/// The offset in the mapped file. If no file is mapped, this field is not relevant
	off: u64,

	// TODO use a sparse array?
	/// The list of allocated physical pages
	pub(super) anon_pages: Vec<Option<Arc<AnonPage>>>,
}

impl MemMapping {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `addr` is the pointer on the virtual memory to the beginning of the mapping. This pointer
	///   must be page-aligned
	/// - `size` is the size of the mapping in pages. The size must be greater than 0
	/// - `prot` is the memory protection
	/// - `flags` the mapping's flags
	/// - `file` is the open file the mapping points to. If `None`, no file is mapped
	/// - `off` is the offset in `file`, if applicable
	pub fn new(
		addr: *mut u8,
		size: NonZeroUsize,
		prot: u8,
		flags: u8,
		file: Option<Arc<File>>,
		off: u64,
	) -> AllocResult<Self> {
		debug_assert!(addr.is_aligned_to(PAGE_SIZE));
		let mut anon_pages = Vec::new();
		anon_pages.resize(size.get(), None)?;
		Ok(Self {
			addr,
			size,
			prot,
			flags,

			file,
			off,

			anon_pages,
		})
	}

	/// Returns a pointer on the virtual memory to the beginning of the mapping.
	pub fn get_addr(&self) -> *mut u8 {
		self.addr
	}

	/// Returns the size of the mapping in memory pages.
	pub fn get_size(&self) -> NonZeroUsize {
		self.size
	}

	/// Returns memory protection.
	pub fn get_prot(&self) -> u8 {
		self.prot
	}

	/// Returns the mapping's flags.
	pub fn get_flags(&self) -> u8 {
		self.flags
	}

	/// Tells whether the given `page` is in COW mode.
	///
	/// An offset is in COW mode if the mapping is not shared, and the number of references to the
	/// page at this offset is higher than `1`.
	///
	/// `flags` is the set of flags of the mapping.
	fn is_cow(phys_page: &Arc<AnonPage>, flags: u8) -> bool {
		if flags & MAP_SHARED != 0 {
			return false;
		}
		// Check if currently shared
		Arc::strong_count(phys_page) > 1
	}

	/// Returns virtual memory context flags.
	///
	/// If `write` is `false, write is disabled even if enabled on the mapping.
	fn vmem_flags(&self, write: bool) -> paging::Entry {
		let mut flags = 0;
		if write && self.prot & PROT_WRITE != 0 {
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			{
				flags |= paging::FLAG_WRITE;
			}
		}
		// Careful, the condition is inverted here. Using == instead of !=
		if self.flags & PROT_EXEC == 0 {
			#[cfg(target_arch = "x86_64")]
			{
				flags |= paging::FLAG_XD;
			}
		}
		flags
	}

	/// If the offset `offset` is pending for an allocation, forces an allocation of a physical
	/// page for that offset.
	///
	/// An offset in a mapping is pending for an allocation if any of the following is true:
	/// - no physical page has been assigned to it other than the default (`page` is `None`)
	/// - the offset is in Copy-On-Write mode
	///
	/// The function also applies the mapping of the page to the given `vmem_transaction`
	/// (regardless of whether the page was effectively in COW mode).
	pub(super) fn alloc(
		&mut self,
		offset: usize,
		vmem_transaction: &mut VMemTransaction<false>,
	) -> AllocResult<()> {
		let virtaddr = VirtAddr::from(self.addr) + offset * PAGE_SIZE;
		// Get previous page
		let previous = self
			.anon_pages
			// Bound check
			.get(offset)
			.ok_or(AllocError)?;
		match previous {
			// If not pending for an allocation: map and stop here
			Some(physaddr) if !Self::is_cow(physaddr, self.flags) => {
				let flags = self.vmem_flags(true);
				return vmem_transaction.map(physaddr.get(), virtaddr, flags);
			}
			_ => {}
		}
		// Allocate and map new page
		let new = Arc::new(AnonPage::new(ZONE_USER)?)?;
		// Tells initializing the new page is necessary
		let init = self.file.is_none();
		// Tells whether a copy from the previous page is necessary
		let copy = previous.is_some();
		if init {
			if let Some(previous) = &previous {
				// Map previous page for copy
				vmem_transaction.map(previous.get(), COPY_BUFFER, 0)?;
			}
		}
		// Map new page
		let new_physaddr = new.get();
		// If the page has to be initialized, do not allow writing during initialization to avoid
		// concurrency issues
		let flags = self.vmem_flags(!init);
		vmem_transaction.map(new_physaddr, virtaddr, flags)?;
		if !init {
			return Ok(());
		}
		// Initialize the new page
		unsafe {
			let dest = self.addr.add(offset * PAGE_SIZE) as *mut Page;
			// Switch to make sure the right vmem is bound, but this should already be the case
			// so consider this has no cost
			vmem::switch(vmem_transaction.vmem, move || {
				vmem::write_ro(|| {
					vmem::smap_disable(|| {
						let dest = &mut *dest;
						if copy {
							dest.copy_from_slice(&*COPY_BUFFER.as_ptr::<Page>());
						} else {
							dest.fill(0);
						}
					});
				});
			});
		}
		// Store the new page and drop the previous
		self.anon_pages[offset] = Some(new);
		// Make the new page writable if necessary. Does not fail since the page has already been
		// mapped
		let flags = self.vmem_flags(true);
		vmem_transaction.map(new_physaddr, virtaddr, flags).unwrap();
		Ok(())
	}

	/// Applies the mapping to the given `vmem_transaction`.
	pub fn apply_to(&mut self, vmem_transaction: &mut VMemTransaction<false>) -> AllocResult<()> {
		if self.flags & MAP_SHARED == 0 {
			for (offset, phys_page) in self.anon_pages.iter().enumerate() {
				let (physaddr, write) = phys_page
					.as_ref()
					.map(|physaddr| {
						let write = !Self::is_cow(physaddr, self.flags);
						(physaddr.get(), write)
					})
					.unwrap_or((zeroed_page(), false));
				let virtaddr = VirtAddr::from(self.addr) + offset * PAGE_SIZE;
				let flags = self.vmem_flags(write);
				vmem_transaction.map(physaddr, virtaddr, flags)?;
				// TODO invalidate cache for this page
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
		let prev = NonZeroUsize::new(begin)
			.map(|size| {
				Ok(MemMapping {
					addr: self.addr,
					size,
					prot: self.prot,
					flags: self.flags,

					file: self.file.clone(),
					off: self.off,

					anon_pages: Vec::try_from(&self.anon_pages[..size.get()])?,
				})
			})
			.transpose()?;
		let gap = NonZeroUsize::new(size).map(|size| {
			let addr = VirtAddr::from(self.addr) + begin * PAGE_SIZE;
			MemGap::new(addr, size)
		});
		// The gap's end
		let end = begin + size;
		let next = self
			.size
			.get()
			.checked_sub(end)
			.and_then(NonZeroUsize::new)
			.map(|size| {
				Ok(Self {
					addr: self.addr.wrapping_add(end * PAGE_SIZE),
					size,
					prot: self.prot,
					flags: self.flags,

					file: self.file.clone(),
					off: self.off + end as u64,

					anon_pages: Vec::try_from(&self.anon_pages[end..])?,
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
	/// If the mapping is locked, the function returns [`utils::errno::EBUSY`].
	pub fn fs_sync(&self, _vmem: &VMem) -> EResult<()> {
		if self.flags & MAP_ANONYMOUS != 0 {
			return Ok(());
		}
		// TODO if locked, EBUSY
		// Get file
		let Some(_file) = &self.file else {
			return Ok(());
		};
		// TODO iterate on pages to look for dirty ones, then write them back to disk
		Ok(())
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
		let addr = VirtAddr::from(self.addr) + pages_range.start * PAGE_SIZE;
		let len = pages_range.end - pages_range.start;
		vmem_transaction.unmap_range(addr, len)?;
		Ok(())
	}
}

impl TryClone for MemMapping {
	fn try_clone(&self) -> AllocResult<Self> {
		Ok(Self {
			addr: self.addr,
			size: self.size,
			prot: self.prot,
			flags: self.flags,

			file: self.file.clone(),
			off: self.off,

			anon_pages: self.anon_pages.try_clone()?,
		})
	}
}
