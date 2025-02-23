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
		buddy::ZONE_USER,
		vmem,
		vmem::{VMem, VMemTransaction},
		PhysAddr, RcPage, VirtAddr,
	},
	process::mem_space::{
		Page, COPY_BUFFER, MAP_ANONYMOUS, MAP_PRIVATE, MAP_SHARED, PROT_EXEC, PROT_WRITE,
	},
};
use core::{num::NonZeroUsize, ops::Range};
use utils::{
	collections::vec::Vec,
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
	TryClone,
};

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

/// Returns virtual memory context flags.
///
/// Arguments:
/// - `prot` is the memory protection
/// - `cow` tells whether we are pending Copy-On-Write
fn vmem_flags(prot: u8, cow: bool) -> paging::Entry {
	let mut flags = 0;
	if !cow && prot & PROT_WRITE != 0 {
		#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
		{
			flags |= paging::FLAG_WRITE;
		}
	}
	// Careful, the condition is inverted here. Using == instead of !=
	if prot & PROT_EXEC == 0 {
		#[cfg(target_arch = "x86_64")]
		{
			flags |= paging::FLAG_XD;
		}
	}
	flags
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
	pub(super) anon_pages: Vec<Option<RcPage>>,
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
		Ok(Self {
			addr,
			size,
			prot,
			flags,

			file,
			off,

			anon_pages: Vec::new(),
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

	/// If the offset `offset` is pending for an allocation, forces an allocation of a physical
	/// page for that offset.
	///
	/// **Note**: it is assumed the associated virtual memory is bound.
	///
	/// If a file is associated with the mapping, the function uses the page cache's content
	/// (potentially populating it by reading from the disk).
	pub(super) fn alloc(
		&mut self,
		offset: usize,
		vmem_transaction: &mut VMemTransaction<false>,
	) -> AllocResult<()> {
		let virtaddr = VirtAddr::from(self.addr) + offset * PAGE_SIZE;
		// If an anonymous page is already present, use it
		let anon_page = self.anon_pages.get(offset).and_then(Option::as_ref);
		if let Some(page) = anon_page {
			if self.flags & MAP_PRIVATE != 0 && page.is_shared() {
				// The page cannot be shared: we need our own copy
				let anon_page = RcPage::new(ZONE_USER)?;
				let anon_physaddr = anon_page.phys_addr();
				// Copy data
				vmem_transaction.map(anon_physaddr, COPY_BUFFER, 0)?;
				vmem::invalidate_page_current(COPY_BUFFER);
				unsafe {
					let src = &*virtaddr.as_ptr::<Page>();
					let dst = &mut *COPY_BUFFER.as_ptr::<Page>();
					vmem::write_ro(|| {
						dst.copy_from_slice(src);
					});
				}
				// Map the page
				let flags = vmem_flags(self.prot, false);
				vmem_transaction
					.map(anon_physaddr, virtaddr, flags)
					.unwrap();
				self.anon_pages[offset] = Some(anon_page);
			} else {
				// Nothing to do, just map the page
				let flags = vmem_flags(self.prot, false);
				return vmem_transaction.map(page.phys_addr(), virtaddr, flags);
			}
		}
		// Else, allocate a page
		match &self.file {
			// Anonymous mapping
			None => {
				// TODO: what if there is already a page and the mapping is private?
				let page = RcPage::new(ZONE_USER)?;
				let physaddr = page.phys_addr();
				// Zero page
				vmem_transaction.map(physaddr, COPY_BUFFER, 0)?;
				vmem::invalidate_page_current(COPY_BUFFER);
				unsafe {
					let buf = &mut *COPY_BUFFER.as_ptr::<Page>();
					vmem::write_ro(|| {
						buf.fill(0);
					});
				}
				// Map the page
				let flags = vmem_flags(self.prot, false);
				vmem_transaction.map(physaddr, virtaddr, flags).unwrap();
				self.anon_pages[offset] = Some(page);
			}
			// Mapped file
			Some(file) => {
				// Get page from file
				let node = file.node().unwrap();
				let pages = node.pages.lock();
				let page = pages.get(offset).and_then(Option::as_ref);
				if page.is_none() {
					// The page is not in cache, read it from disk
					todo!()
				}
				// cannot fail since we just insert the page in cache if it was not present
				let file_page = pages.get(offset).and_then(Option::as_ref).unwrap();
				let file_physaddr = file_page.phys_addr();
				if self.flags & MAP_PRIVATE != 0 {
					let anon_page = RcPage::new(ZONE_USER)?;
					let anon_physaddr = anon_page.phys_addr();
					// Copy data
					vmem_transaction.map(anon_physaddr, COPY_BUFFER, 0)?;
					vmem::invalidate_page_current(COPY_BUFFER);
					unsafe {
						let src = &*file_page.virt_addr().as_ptr::<Page>();
						let dst = &mut *COPY_BUFFER.as_ptr::<Page>();
						vmem::write_ro(|| {
							dst.copy_from_slice(src);
						});
					}
					// Map the page
					let flags = vmem_flags(self.prot, false);
					vmem_transaction
						.map(anon_physaddr, virtaddr, flags)
						.unwrap();
					self.anon_pages[offset] = Some(anon_page);
				} else {
					// Just use the file's page
					let flags = vmem_flags(self.prot, false);
					vmem_transaction
						.map(file_physaddr, virtaddr, flags)
						.unwrap();
				}
			}
		}
		Ok(())
	}

	/// Applies the mapping to the given `vmem_transaction`.
	pub fn apply_to(&mut self, vmem_transaction: &mut VMemTransaction<false>) -> AllocResult<()> {
		let shared = self.flags & MAP_SHARED != 0;
		let mut file_pages = self.file.as_mut().map(|file| {
			// cannot fail since a mapped file always has an associated Node
			let node = file.node().unwrap();
			node.pages.lock()
		});
		for off in 0..self.size.get() {
			// Get page
			let anon_page = self.anon_pages.get(off).and_then(Option::as_ref);
			let page = if let Some(page) = anon_page {
				Some(page)
			} else if let Some(file_pages) = &mut file_pages {
				let page = file_pages.get(off).and_then(Option::as_ref);
				if page.is_none() {
					// The page is not in cache, read it from disk
					todo!()
				}
				file_pages.get(off).and_then(Option::as_ref)
			} else {
				None
			};
			// Map
			let (physaddr, pending_cow) = page
				.map(|p| (p.phys_addr(), !shared && p.is_shared()))
				.unwrap_or((zeroed_page(), true));
			let virtaddr = VirtAddr::from(self.addr) + off * PAGE_SIZE;
			let flags = vmem_flags(self.prot, pending_cow);
			vmem_transaction.map(physaddr, virtaddr, flags)?;
			// TODO invalidate cache for this page
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
