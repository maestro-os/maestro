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
		PhysAddr, VirtAddr,
		buddy::ZONE_USER,
		cache::{FrameOwner, RcFrame},
		vmem::{VMem, shootdown_page, write_ro},
	},
	process::mem_space::{
		COPY_BUFFER, MAP_ANONYMOUS, MAP_PRIVATE, MAP_SHARED, MemSpace, PROT_EXEC, PROT_WRITE, Page,
	},
	sync::spin::Spin,
	time::clock::{Clock, current_time_ms},
};
use core::{num::NonZeroUsize, ops::Deref, sync::atomic::Ordering::Release};
use utils::{
	TryClone,
	collections::vec::Vec,
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	ptr::arc::Arc,
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

/// A wrapper for a mapped frame, allowing to update the map counter.
#[derive(Debug)]
pub(super) struct MappedFrame(RcFrame);

impl MappedFrame {
	/// Creates a new instance.
	pub fn new(frame: RcFrame) -> Self {
		frame.map_counter().fetch_add(1, Release);
		Self(frame)
	}
}

impl Deref for MappedFrame {
	type Target = RcFrame;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl Clone for MappedFrame {
	fn clone(&self) -> Self {
		Self::new(self.0.clone())
	}
}

impl Drop for MappedFrame {
	fn drop(&mut self) {
		self.0.map_counter().fetch_sub(1, Release);
	}
}

/// Returns virtual memory context flags.
///
/// Arguments:
/// - `prot` is the memory protection
/// - `cow` tells whether we are pending Copy-On-Write
fn vmem_flags(prot: u8, cow: bool) -> usize {
	let mut flags = paging::FLAG_USER;
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

// FIXME: SMAP and mapping the page to userspace before init (potential data leak to userspace)
/// Initializes a new page and maps it at `dst`.
///
/// Arguments:
/// - `vmem` is the transaction on which the page mapping takes place
/// - `prot` is the memory protection for the newly mapped page
/// - `src` is the page containing the data to initialize the new page with. If `None`, the new
///   page is initialized with zeros
/// - `dst` is the virtual address at which the new page is mapped
fn init_page(vmem: &VMem, prot: u8, src: Option<&RcFrame>, dst: VirtAddr) -> AllocResult<RcFrame> {
	// Allocate destination page
	let new_page = RcFrame::new(0, ZONE_USER, FrameOwner::Anon, 0)?;
	// Map source page to copy buffer if any
	if let Some(src) = src {
		vmem.map(src.phys_addr(), COPY_BUFFER, 0);
	}
	// Map destination page
	let flags = vmem_flags(prot, false);
	vmem.map(new_page.phys_addr(), dst, flags);
	// Copy or zero
	unsafe {
		// Required since the copy buffer is mapped without write permission
		write_ro(|| {
			let src = src.is_some().then_some(&*COPY_BUFFER.as_ptr::<Page>());
			let dst = &mut *dst.as_ptr::<Page>();
			if let Some(src) = src {
				dst.copy_from_slice(src);
			} else {
				dst.fill(0);
			}
		});
	}
	Ok(new_page)
}

/// A mapping in a memory space.
#[derive(Debug)]
pub struct MemMapping {
	/// Address on the virtual memory to the beginning of the mapping
	pub addr: VirtAddr,
	/// The size of the mapping in pages
	pub size: NonZeroUsize,
	/// Memory protection
	pub prot: u8,
	/// Mapping flags
	pub flags: i32,

	/// The mapped file, if any
	pub file: Option<Arc<File>>,
	/// The offset in the mapped file. If no file is mapped, this field is not relevant
	pub off: u64,

	// TODO use a sparse array?
	/// The list of allocated physical pages
	pub(super) pages: Spin<Vec<Option<MappedFrame>>>,
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
	/// - `file` is the mapped file. If `None`, no file is mapped
	/// - `off` is the offset in `file`, if applicable
	pub(super) fn new(
		addr: VirtAddr,
		size: NonZeroUsize,
		prot: u8,
		flags: i32,
		file: Option<Arc<File>>,
		off: u64,
	) -> AllocResult<Self> {
		debug_assert!(addr.is_aligned_to(PAGE_SIZE));
		let mut pages = Vec::new();
		pages.resize(size.get(), None)?;
		Ok(Self {
			addr,
			size,
			prot,
			flags,

			file,
			off,

			pages: Spin::new(pages),
		})
	}

	/// Maps the page at the offset `offset` of the mapping, onto `mem_space`.
	///
	/// `write` tells whether the page has to be mapped for writing.
	///
	/// If no underlying physical memory exist for this offset, the function might allocate it.
	///
	/// **Note**: it is assumed the associated virtual memory is bound.
	///
	/// If a file is mapped, the function uses the page cache's content (potentially populating it
	/// by reading from the disk).
	///
	/// Upon allocation failure, or failure to read a page from the disk, the function returns an
	/// error.
	pub(super) fn map(&self, mem_space: &MemSpace, offset: usize, write: bool) -> EResult<()> {
		let virtaddr = self.addr + offset * PAGE_SIZE;
		let mut pages = self.pages.lock();
		if let Some(page) = &pages[offset] {
			// A page is already present, use it
			let mut phys_addr = page.phys_addr();
			let pending_cow = self.flags & MAP_SHARED == 0 && page.is_shared();
			if pending_cow {
				// The page cannot be shared: we need our own copy (regardless of whether we are
				// reading or writing)
				let page = init_page(&mem_space.vmem, self.prot, Some(page), virtaddr)?;
				phys_addr = page.phys_addr();
				pages[offset] = Some(MappedFrame::new(page));
			}
			// Map the page
			let flags = vmem_flags(self.prot, false);
			mem_space.vmem.map(phys_addr, virtaddr, flags);
			return Ok(());
		}
		// Else, Allocate a page
		match &self.file {
			// Anonymous mapping
			None => {
				let phys_addr = if write {
					let page = init_page(&mem_space.vmem, self.prot, None, virtaddr)?;
					let phys_addr = page.phys_addr();
					pages[offset] = Some(MappedFrame::new(page));
					phys_addr
				} else {
					// Lazy allocation: map the zeroed page
					zeroed_page()
				};
				// Map
				let flags = vmem_flags(self.prot, !write);
				mem_space.vmem.map(phys_addr, virtaddr, flags);
			}
			// Mapped file
			Some(file) => {
				// Get page from file
				let node = file.node().unwrap();
				let file_off = self.off / PAGE_SIZE as u64 + offset as u64;
				let mut page = node.node_ops.read_page(node, file_off)?;
				// If the mapping is private, we need our own copy
				if self.flags & MAP_PRIVATE != 0 {
					page = init_page(&mem_space.vmem, self.prot, Some(&page), virtaddr)?;
				}
				let phys_addr = page.phys_addr();
				pages[offset] = Some(MappedFrame::new(page));
				// Map
				let flags = vmem_flags(self.prot, !write);
				mem_space.vmem.map(phys_addr, virtaddr, flags);
			}
		}
		shootdown_page(virtaddr, mem_space.bound_cpus());
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
	pub(super) fn split(
		&self,
		begin: usize,
		size: usize,
	) -> AllocResult<(Option<Self>, Option<MemGap>, Option<Self>)> {
		let pages = self.pages.lock();
		let prev = NonZeroUsize::new(begin)
			.map(|size| {
				Ok(MemMapping {
					addr: self.addr,
					size,
					prot: self.prot,
					flags: self.flags,

					file: self.file.clone(),
					off: self.off,

					pages: Spin::new(Vec::try_from(&pages[..size.get()])?),
				})
			})
			.transpose()?;
		let gap = NonZeroUsize::new(size).map(|size| {
			let addr = self.addr + begin * PAGE_SIZE;
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
					addr: self.addr + end * PAGE_SIZE,
					size,
					prot: self.prot,
					flags: self.flags,

					file: self.file.clone(),
					off: self.off + end as u64,

					pages: Spin::new(Vec::try_from(&pages[end..])?),
				})
			})
			.transpose()?;
		Ok((prev, gap, next))
	}

	/// Synchronizes the data on the memory mapping back to the filesystem.
	///
	/// Arguments:
	/// - `vmem` is the virtual memory context
	/// - `sync` tells whether the synchronization should be performed synchronously
	///
	/// The function does nothing if:
	/// - The mapping is not shared
	/// - The mapping is not associated with a file
	///
	/// If the mapping is locked, the function returns [`utils::errno::EBUSY`].
	pub(super) fn sync(&self, vmem: &VMem, sync: bool) -> EResult<()> {
		if self.flags & (MAP_ANONYMOUS | MAP_PRIVATE) != 0 {
			return Ok(());
		}
		// TODO if locked, EBUSY
		if self.file.is_none() {
			return Ok(());
		}
		let ts = current_time_ms(Clock::Boottime);
		let pages = self.pages.lock();
		for frame in pages.iter().flatten() {
			vmem.poll_dirty(self.addr, self.size.get());
			if sync {
				// TODO warn on error?
				let _ = frame.writeback(Some(ts), false);
			}
		}
		Ok(())
	}
}

impl TryClone for MemMapping {
	fn try_clone(&self) -> AllocResult<Self> {
		let pages = self.pages.lock();
		Ok(Self {
			addr: self.addr,
			size: self.size,
			prot: self.prot,
			flags: self.flags,

			file: self.file.clone(),
			off: self.off,

			pages: Spin::new(pages.try_clone()?),
		})
	}
}
