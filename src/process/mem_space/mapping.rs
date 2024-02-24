//! A memory mapping is a region of virtual memory that a process can access.
//!
//! Mappings may be created at the process's creation or by the process itself using
//! system calls.

use super::{gap::MemGap, MapResidence};
use crate::{
	file::vfs,
	memory,
	memory::{
		physical_ref_counter::PhysRefCounter,
		vmem,
		vmem::{VMem, VMemTransaction},
	},
	process::{AllocResult, EResult},
	util::io::IO,
};
use core::{ffi::c_void, fmt, num::NonZeroUsize, ops::Range, ptr, slice};

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
#[derive(Clone)]
pub struct MemMapping {
	/// Address on the virtual memory to the beginning of the mapping
	begin: *mut c_void,
	/// The size of the mapping in pages.
	size: NonZeroUsize,
	/// The mapping's flags.
	flags: u8,
	/// The residence of the mapping.
	residence: MapResidence,
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
	) -> Self {
		debug_assert!(begin.is_aligned_to(memory::PAGE_SIZE));
		Self {
			begin,
			size,
			flags,
			residence,
		}
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

	/// Returns a pointer to the physical page of memory associated with the
	/// mapping at page offset `offset`.
	///
	/// If no page is associated, the function returns `None`.
	fn get_physical_page(&self, offset: usize, vmem: &VMem) -> Option<*const c_void> {
		if offset >= self.size.get() {
			return None;
		}

		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		let phys_ptr = vmem.translate(virt_ptr)?;
		if phys_ptr != get_default_page() {
			Some(phys_ptr)
		} else {
			None
		}
	}

	/// Tells whether the page at offset `offset` in the mapping is shared with another mapping on
	/// the system or not.
	fn is_shared(&self, offset: usize, vmem: &VMem) -> bool {
		let Some(phys_ptr) = self.get_physical_page(offset, vmem) else {
			return false;
		};
		let ref_counter = super::PHYSICAL_REF_COUNTER.lock();
		ref_counter.is_shared(phys_ptr)
	}

	/// Tells whether the page at offset `offset` is waiting for Copy-On-Write.
	fn is_cow(&self, offset: usize, vmem: &VMem) -> bool {
		self.flags & super::MAPPING_FLAG_SHARED == 0
			&& self.residence.is_normal()
			&& self.is_shared(offset, vmem)
	}

	/// Returns the flags for the virtual memory context for the given virtual page offset.
	///
	/// Arguments:
	/// - `allocated` tells whether the page has been physically allocated.
	/// - `offset` is the offset of the page in the mapping.
	fn get_vmem_flags(&self, allocated: bool, offset: usize, vmem: &VMem) -> u32 {
		let mut flags = 0;
		if self.flags & super::MAPPING_FLAG_WRITE != 0 && allocated && !self.is_cow(offset, vmem) {
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
	/// `vmem` is the affected virtual memory context.
	///
	/// If data is already present on the mapping at this offset, the function copies it to the
	/// newly allocated page.
	///
	/// On success, the function returns the transaction.
	pub fn alloc(&self, offset: usize, vmem: &mut VMem) -> AllocResult<VMemTransaction<false>> {
		let mut transaction = vmem.transaction();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *mut c_void;
		let cow_buffer = {
			if self.is_cow(offset, vmem) {
				let mut cow_buffer = crate::vec![0u8; memory::PAGE_SIZE]?;
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

		let prev_phys_ptr = self.get_physical_page(offset, vmem);
		if self.residence.is_normal() && cow_buffer.is_none() && prev_phys_ptr.is_some() {
			return Ok(transaction);
		}

		// Map new page
		let new_phys_ptr = self.residence.alloc_page(offset)?;
		let flags = self.get_vmem_flags(true, offset, vmem);
		let res = transaction.map(new_phys_ptr.as_ptr(), virt_ptr, flags);
		if let Err(e) = res {
			self.residence.free_page(offset, new_phys_ptr.as_ptr());
			return Err(e);
		}

		// Free previous page
		if let Some(prev_phys_ptr) = prev_phys_ptr {
			self.residence.free_page(offset, prev_phys_ptr);
		}

		// Copy data if necessary
		if self.residence.is_normal() {
			unsafe {
				// FIXME: switching vmem at each call to `map` is suboptimal (try to batch)
				vmem::switch(vmem, move || {
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

		Ok(transaction)
	}

	/// Maps the mapping to `vmem` with the default page.
	///
	/// If the mapping is marked as no-lazy, the function allocates physical memory and maps it
	/// instead of the default page.
	///
	/// On success, the function returns the transaction.
	pub fn map_default(&self, vmem: &mut VMem) -> AllocResult<VMemTransaction<false>> {
		let mut transaction = vmem.transaction();
		let lazy = self.flags & super::MAPPING_FLAG_NOLAZY == 0;
		let use_default = lazy && self.residence.is_normal();
		let size = self.size.get();
		if use_default {
			let default_page = get_default_page();
			for i in 0..size {
				let virtaddr = unsafe { self.begin.add(i * memory::PAGE_SIZE) };
				let flags = self.get_vmem_flags(false, i, vmem);
				transaction.map(default_page, virtaddr, flags)?;
			}
		} else {
			for i in 0..size {
				self.alloc(i, transaction.vmem)?;
			}
		}
		Ok(transaction)
	}

	/// Frees the physical page stored in `vmem` at the offset `offset` of the mapping.
	///
	/// If the page is shared, it is not freed but the reference counter is decreased.
	///
	/// # Safety
	///
	/// Accessing a mapping whose physical pages have been freed has an undefined behaviour.
	pub(super) unsafe fn free_phys_page(&self, offset: usize, vmem: &VMem) {
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			if phys_ptr == get_default_page() {
				return;
			}
			self.residence.free_page(offset, phys_ptr);
		}
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
	) -> (Option<Self>, Option<MemGap>, Option<Self>) {
		let begin_ptr = unsafe { self.begin.add(begin * memory::PAGE_SIZE) };
		let prev = NonZeroUsize::new(begin).map(|begin| MemMapping {
			begin: self.begin,
			size: begin,
			flags: self.flags,
			residence: self.residence.clone(),
		});
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
				Self {
					begin,
					size,
					flags: self.flags,
					residence,
				}
			});
		(prev, gap, next)
	}

	/// Updates `vmem` according to the mapping for the page at offset `offset`.
	pub fn update_vmem(&self, offset: usize, vmem: &mut VMem) {
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			let allocated = phys_ptr != get_default_page();
			let flags = self.get_vmem_flags(allocated, offset, vmem);
			// Cannot fail because the page for the vmem structure is already mapped
			vmem.map(phys_ptr, virt_ptr, flags).unwrap();
		}
	}

	/// After a fork operation failed, frees the pages that were already
	/// allocated.
	///
	/// `n` is the number of pages to free from the beginning.
	fn fork_fail_clean(&self, ref_counter: &mut PhysRefCounter, n: usize, vmem: &VMem) {
		for i in 0..n {
			if let Some(phys_ptr) = self.get_physical_page(i, vmem) {
				ref_counter.decrement(phys_ptr);
			}
		}
	}

	/// Clones the mapping for the fork operation. The new mapping is sharing
	/// the same physical memory, for Copy-On-Write.
	///
	/// `vmem` is the virtual memory context for the new mapping
	///
	/// The virtual memory context has to be updated after calling this
	/// function.
	///
	/// The function returns then newly created mapping.
	pub(super) fn fork(&self, vmem: &mut VMem) -> AllocResult<Self> {
		let new_mapping = Self {
			begin: self.begin,
			size: self.size,
			flags: self.flags,
			residence: self.residence.clone(),
		};
		let nolazy = new_mapping.flags & super::MAPPING_FLAG_NOLAZY != 0;
		if nolazy {
			for i in 0..self.size.get() {
				unsafe {
					let virtaddr = self.begin.add(i * memory::PAGE_SIZE);
					vmem.unmap(virtaddr)?;
				}
				new_mapping.alloc(i, vmem)?;
			}
		} else {
			let mut ref_counter = super::PHYSICAL_REF_COUNTER.lock();
			for i in 0..self.size.get() {
				let Some(phys_ptr) = self.get_physical_page(i, vmem) else {
					continue;
				};
				if let Err(errno) = ref_counter.increment(phys_ptr) {
					self.fork_fail_clean(&mut ref_counter, i, vmem);
					return Err(errno);
				}
			}
		}
		Ok(new_mapping)
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

	/// Unmaps the mapping from `vmem`.
	///
	/// `range` is the range of pages affect by the unmap. Pages outside of this range are left
	/// untouched.
	///
	/// If applicable, the function synchronizes the data on the pages to be unmapped to the disk.
	///
	/// If the associated physical pages are not shared, the function frees them.
	///
	/// This function doesn't flush the virtual memory context.
	///
	/// On success, the function returns the transaction.
	pub fn unmap(
		&self,
		pages_range: Range<usize>,
		vmem: &mut VMem,
	) -> EResult<VMemTransaction<false>> {
		// Synchronize to disk
		self.fs_sync(vmem)?;
		let begin_ptr = unsafe { self.begin.add(pages_range.start * memory::PAGE_SIZE) };
		// Unmap virtual pages
		let mut transaction = vmem.transaction();
		transaction.unmap_range(begin_ptr, pages_range.end - pages_range.start)?;
		// Remove physical pages
		// FIXME: do not free here as the mappings might be reused on rollback
		// TODO: instead, store pointers to physical pages in the mapping itself and free when
		// dropping the mapping
		for i in pages_range {
			// Safety: virtual pages have been unmapped just before
			unsafe {
				self.free_phys_page(i, vmem);
			}
		}
		Ok(transaction)
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
