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

use super::{gap::MemGap, MapResidence};
use crate::{
	file::vfs,
	memory,
	memory::{
		vmem,
		vmem::{VMem, VMemTransaction},
	},
	process::{AllocResult, EResult},
	util::{collections::vec::Vec, io::IO, ptr::arc::Arc, TryClone},
	vec,
};
use core::{ffi::c_void, fmt, mem, num::NonZeroUsize, ops::Range, ptr, ptr::NonNull, slice};

/// Returns a physical pointer to the default page.
///
/// This page is meant to be mapped in read-only and is a placeholder for pages that are
/// accessed without being allocated nor written.
#[inline]
fn get_default_page() -> *const c_void {
	#[repr(align(4096))]
	struct DefaultPage([u8; memory::PAGE_SIZE]);
	static DEFAULT_PAGE: DefaultPage = DefaultPage([0; memory::PAGE_SIZE]);
	memory::kern_to_phys(DEFAULT_PAGE.0.as_ptr() as _)
}

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
	phys_pages: Vec<Option<Arc<NonNull<[u8; memory::PAGE_SIZE]>>>>,
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

	/// Tells whether the page at offset `offset` in the mapping is shared with another mapping on
	/// the system or not.
	fn is_shared(&self, offset: usize) -> bool {
		self.phys_pages[offset]
			.as_ref()
			.map(|a| Arc::strong_count(a) > 1)
			.unwrap_or(false)
	}

	/// Tells whether the page at offset `offset` is waiting for Copy-On-Write.
	fn is_cow(&self, offset: usize) -> bool {
		self.flags & super::MAPPING_FLAG_SHARED == 0
			&& self.residence.is_normal()
			&& self.is_shared(offset)
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

	/// Allocates physical pages dedicated to the mapping at offset `offset`.
	///
	/// `vmem_transaction` is the transaction in which the mapping is done.
	///
	/// If data is already present on the mapping at this offset, the function copies it to the
	/// newly allocated page.
	pub fn alloc(
		&self,
		offset: usize,
		vmem_transaction: &mut VMemTransaction<false>,
	) -> AllocResult<()> {
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *mut c_void;
		let cow_buffer = {
			if self.is_cow(offset) {
				let mut cow_buffer = vec![0u8; memory::PAGE_SIZE]?;
				unsafe {
					ptr::copy_nonoverlapping(
						virt_ptr,
						cow_buffer.as_mut_slice().as_mut_ptr() as _,
						memory::PAGE_SIZE,
					);
				}
				Some(cow_buffer)
			} else {
				None
			}
		};

		let prev_phys_ptr = self.phys_pages[offset].clone();
		if self.residence.is_normal() && cow_buffer.is_none() && prev_phys_ptr.is_some() {
			return Ok(());
		}

		// Map new page
		let new_phys_ptr = self.residence.alloc_page(offset)?;
		let flags = self.get_vmem_flags(true, offset);
		let res = vmem_transaction.map(new_phys_ptr.cast().as_ptr(), virt_ptr, flags);
		if let Err(e) = res {
			unsafe {
				self.residence.free_page(offset, new_phys_ptr);
			}
			return Err(e);
		}

		// Free previous page
		if let Some(prev_phys_ptr) = prev_phys_ptr.and_then(Arc::into_inner) {
			unsafe {
				self.residence.free_page(offset, prev_phys_ptr);
			}
		}

		// Copy data if necessary
		if self.residence.is_normal() {
			unsafe {
				// FIXME: switching vmem at each call to `map` is suboptimal (try to batch)
				vmem::switch(vmem_transaction.vmem, move || {
					vmem::write_lock_wrap(|| {
						if let Some(buffer) = cow_buffer {
							ptr::copy_nonoverlapping(
								buffer.as_ptr() as *const c_void,
								virt_ptr,
								memory::PAGE_SIZE,
							);
						} else {
							// Zero memory
							let slice = slice::from_raw_parts_mut::<u8>(
								virt_ptr as *mut _,
								memory::PAGE_SIZE,
							);
							slice.fill(0);
						}
					});
				});
			}
		}

		Ok(())
	}

	/// Maps the mapping with the default page.
	///
	/// `vmem_transaction` is the transaction in which the mapping is done.
	///
	/// If the mapping is marked as no-lazy, the function allocates physical memory and maps it
	/// instead of the default page.
	pub fn map_default(&self, vmem_transaction: &mut VMemTransaction<false>) -> AllocResult<()> {
		let size = self.size.get();
		if self.residence.is_normal() {
			// Use default page
			let default_page = get_default_page();
			for i in 0..size {
				let virtaddr = (self.begin as usize + (i * memory::PAGE_SIZE)) as *const c_void;
				let flags = self.get_vmem_flags(false, i);
				vmem_transaction.map(default_page, virtaddr, flags)?;
			}
		} else {
			// Allocate directly
			for i in 0..size {
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
		let begin_ptr = unsafe { self.begin.add(begin * memory::PAGE_SIZE) };
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
				let begin = unsafe { self.begin.add(end * memory::PAGE_SIZE) };
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

	/// Forks the mappings.
	///
	/// `vmem_transactions` is a set of two transactions to update.
	///
	/// Both transactions are remapped so that the mapping is present on both and is made read-only
	/// for Copy-On-Write.
	pub fn fork(
		&mut self,
		mut vmem_transactions: [&mut VMemTransaction<false>; 2],
	) -> AllocResult<Self> {
		// Clone physical pages references to make them shared
		let phys_pages = self.phys_pages.try_clone()?;
		// Init Copy-On-Write by marking mappings as read-only
		let default_page = get_default_page();
		for (i, phys_page) in phys_pages.iter().enumerate() {
			let physaddr = phys_page
				.as_ref()
				.map(|p| p.cast().as_ptr() as *const c_void);
			let allocated = physaddr.is_some();
			let physaddr = physaddr.unwrap_or(default_page);
			let virtaddr = (self.begin as usize + i * memory::PAGE_SIZE) as *const c_void;
			let flags = self.get_vmem_flags(allocated, i);
			// Apply to all given vmem
			for t in &mut vmem_transactions {
				t.map(physaddr, virtaddr, flags)?;
			}
		}
		Ok(Self {
			begin: self.begin,
			size: self.size,
			flags: self.flags,
			residence: self.residence.clone(),

			phys_pages,
		})
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
		let begin = unsafe { self.begin.add(pages_range.start * memory::PAGE_SIZE) };
		let len = pages_range.end - pages_range.start;
		vmem_transaction.unmap_range(begin, len)?;
		Ok(())
	}
}

impl fmt::Debug for MemMapping {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let end = unsafe { self.begin.add(self.size.get() * memory::PAGE_SIZE) };
		write!(
			f,
			"MemMapping {{ begin: {:p}, end: {:p}, flags: {} }}",
			self.begin, end, self.flags,
		)
	}
}

impl Drop for MemMapping {
	fn drop(&mut self) {
		// Free physical pages that are not shared with other mappings
		let phys_pages = mem::take(&mut self.phys_pages);
		phys_pages
			.into_iter()
			.enumerate()
			.filter_map(|(i, p)| Some((i, Arc::into_inner(p?)?)))
			.for_each(|(i, p)| unsafe {
				self.residence.free_page(i, p);
			});
	}
}
