//! A memory mapping is a region of virtual memory that a process can access.
//!
//! Mappings may be created at the process's creation or by the process itself using
//! system calls.

use super::{gap::MemGap, MapResidence};
use crate::{
	file::vfs,
	memory,
	memory::{buddy, physical_ref_counter::PhysRefCounter, vmem, vmem::VMem},
	process::{oom, AllocResult, EResult},
	util::{boxed::Box, io::IO, lock::*, ptr::arc::Arc},
};
use core::{ffi::c_void, fmt, num::NonZeroUsize, ops::Range, ptr, ptr::NonNull, slice};

/// Returns a physical pointer to the default page.
fn get_default_page() -> *const c_void {
	/// The default physical page of memory.
	///
	/// This page is meant to be mapped in read-only and is a placeholder for pages that are
	/// accessed without being allocated nor written.
	static DEFAULT_PAGE: Mutex<Option<NonNull<c_void>>> = Mutex::new(None);
	let mut default_page = DEFAULT_PAGE.lock();
	match &mut *default_page {
		Some(ptr) => ptr.as_ptr(),
		// Lazy allocation
		None => unsafe {
			let Ok(mut ptr) = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_KERNEL) else {
				panic!("Cannot allocate default memory page!");
			};
			// Zero page
			let virt_ptr = memory::kern_to_virt(ptr.as_mut()) as *mut u8;
			let slice = slice::from_raw_parts_mut(virt_ptr, memory::PAGE_SIZE);
			slice.fill(0);
			*default_page = Some(ptr);
			ptr.as_ptr()
		},
	}
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

	/// The virtual memory context handler on which the mapping is present.
	vmem: Arc<Mutex<Box<dyn VMem>>>,
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
	/// - `file` is the open file the mapping points to, with an offset in it.
	/// If `None`, the mapping doesn't point to any file.
	/// - `vmem` is the virtual memory context on which the mapping is present.
	/// - `residence` is the residence for the mapping.
	pub fn new(
		begin: *mut c_void,
		size: NonZeroUsize,
		flags: u8,
		vmem: Arc<Mutex<Box<dyn VMem>>>,
		residence: MapResidence,
	) -> Self {
		debug_assert!(begin.is_aligned_to(memory::PAGE_SIZE));
		Self {
			begin,
			size,
			flags,

			vmem,
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

	/// Returns a reference to the virtual memory context handler associated
	/// with the mapping.
	pub fn get_vmem(&self) -> &Arc<Mutex<Box<dyn VMem>>> {
		&self.vmem
	}

	/// Tells whether the mapping contains the given virtual address `ptr`.
	pub fn contains_ptr(&self, ptr: *const c_void) -> bool {
		// TODO check this is correct regarding LLVM provenances
		ptr >= self.begin && ptr < (self.begin as usize + self.size.get() * memory::PAGE_SIZE) as _
	}

	/// Returns a pointer to the physical page of memory associated with the
	/// mapping at page offset `offset`.
	///
	/// If no page is associated, the function returns `None`.
	fn get_physical_page(&self, offset: usize, vmem: &dyn VMem) -> Option<*const c_void> {
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
	fn is_shared(&self, offset: usize, vmem: &dyn VMem) -> bool {
		let Some(phys_ptr) = self.get_physical_page(offset, vmem) else {
			return false;
		};
		let ref_counter = super::PHYSICAL_REF_COUNTER.lock();
		ref_counter.is_shared(phys_ptr)
	}

	/// Tells whether the page at offset `offset` is waiting for Copy-On-Write.
	fn is_cow(&self, offset: usize, vmem: &dyn VMem) -> bool {
		self.flags & super::MAPPING_FLAG_SHARED == 0
			&& self.residence.is_normal()
			&& self.is_shared(offset, vmem)
	}

	/// Returns the flags for the virtual memory context for the given virtual page offset.
	///
	/// Arguments:
	/// - `allocated` tells whether the page has been physically allocated.
	/// - `offset` is the offset of the page in the mapping.
	fn get_vmem_flags(&self, allocated: bool, offset: usize, vmem: &dyn VMem) -> u32 {
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

	/// Maps the page at offset `offset` in the mapping to the virtual memory
	/// context.
	///
	/// The function allocates the physical memory to be mapped.
	///
	/// If the mapping is in forking state, the function shall apply Copy-On-Write and allocate a
	/// new physical page with the same data.
	///
	/// If a physical page is already mapped, the function does nothing.
	pub fn map(&mut self, offset: usize) -> AllocResult<()> {
		let mut vmem = self.vmem.lock();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *mut c_void;

		let cow_buffer = {
			if self.is_cow(offset, &**vmem) {
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

		let prev_phys_ptr = self.get_physical_page(offset, &**vmem);
		if self.residence.is_normal() && cow_buffer.is_none() && prev_phys_ptr.is_some() {
			return Ok(());
		}

		// Map new page
		let new_phys_ptr = self.residence.alloc_page(offset)?;
		let flags = self.get_vmem_flags(true, offset, &**vmem);
		let res = unsafe { vmem.map(new_phys_ptr.as_ptr(), virt_ptr, flags) };
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
				vmem::switch(&**vmem, move || {
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

	/// Maps the mapping to the given virtual memory context with the default page.
	///
	/// If the mapping is marked as no-lazy, the function allocates physical memory and maps it
	/// instead of the default page.
	///
	/// The default page is dependent on the nature of the mapping's residence.
	pub fn map_default(&mut self) -> AllocResult<()> {
		let use_default =
			self.flags & super::MAPPING_FLAG_NOLAZY == 0 && self.residence.is_normal();
		if use_default {
			let mut vmem = self.vmem.lock();
			let default_page = get_default_page();
			for i in 0..self.size.get() {
				let virtaddr = unsafe { self.begin.add(i * memory::PAGE_SIZE) };
				let flags = self.get_vmem_flags(false, i, &**vmem);
				unsafe {
					vmem.map(default_page, virtaddr, flags)?;
				}
			}
		} else {
			for i in 0..self.size.get() {
				if let Err(errno) = self.map(i) {
					// Cannot fail because `map` is not using the PAGE_SIZE flag, so unmapping
					// won't allocate. FIXME: should not rely on this kind of implementation detail
					let _ = self.unmap(0..(i + 1));
					return Err(errno);
				}
			}
		}
		Ok(())
	}

	/// Frees the physical page at offset `offset` of the mapping.
	///
	/// If the page is shared, it is not freed but the reference counter is decreased.
	fn free_phys_page(&mut self, offset: usize) {
		let vmem = self.vmem.lock();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			if phys_ptr == get_default_page() {
				return;
			}
			self.residence.free_page(offset, phys_ptr);
		}
	}

	/// Unmaps the mapping from the given virtual memory context.
	///
	/// If the physical pages the mapping points to are not shared, the function frees them.
	///
	/// This function doesn't flush the virtual memory context.
	pub fn unmap(&mut self, pages_range: Range<usize>) -> AllocResult<()> {
		// Remove physical pages
		for i in pages_range.clone() {
			self.free_phys_page(i);
		}
		let begin_ptr = unsafe { self.begin.add(pages_range.start * memory::PAGE_SIZE) };
		// Unmap physical pages
		let mut vmem = self.vmem.lock();
		// FIXME: potential deadly loop? the previous calls to `free_phys_page` are freeing
		// physical pages, which *should* be enough to execute the line below, but this is not
		// something we should rely on. there is also no guarantee that nobody is going to use the
		// then-freed memory in between
		oom::wrap(|| unsafe { vmem.unmap_range(begin_ptr, pages_range.end - pages_range.start) });
		Ok(())
	}

	/// Partially unmaps the current mapping, creating up to two new mappings and one gap.
	///
	/// Arguments:
	/// - `begin` is the index of the first page to be unmapped.
	/// - `size` is the number of pages to unmap.
	///
	/// If the region to be unmapped is out of bounds, it is truncated to the end of the mapping.
	///
	/// The newly created mappings correspond to the remaining pages.
	///
	/// The newly created gap is in place of the unmapped portion.
	///
	/// If the mapping is completely unmapped, the function returns no new mappings.
	///
	/// The function doesn't flush the virtual memory context.
	pub fn partial_unmap(
		&self,
		begin: usize,
		size: usize,
	) -> (Option<Self>, Option<MemGap>, Option<Self>) {
		let begin_ptr = unsafe { self.begin.add(begin * memory::PAGE_SIZE) };
		let prev = NonZeroUsize::new(begin).map(|begin| MemMapping {
			begin: self.begin,
			size: begin,
			flags: self.flags,

			vmem: self.vmem.clone(),
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

					vmem: self.vmem.clone(),
					residence,
				}
			});
		(prev, gap, next)
	}

	/// Updates the virtual memory context according to the mapping for the page
	/// at offset `offset`.
	pub fn update_vmem(&mut self, offset: usize) {
		let mut vmem = self.vmem.lock();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			let allocated = phys_ptr != get_default_page();
			let flags = self.get_vmem_flags(allocated, offset, &**vmem);
			// Cannot fail because the page for the vmem structure is already mapped
			unsafe {
				vmem.map(phys_ptr, virt_ptr, flags).unwrap();
			}
		}
	}

	/// After a fork operation failed, frees the pages that were already
	/// allocated.
	///
	/// `n` is the number of pages to free from the beginning.
	fn fork_fail_clean(&self, ref_counter: &mut PhysRefCounter, n: usize, vmem: &dyn VMem) {
		for i in 0..n {
			if let Some(phys_ptr) = self.get_physical_page(i, vmem) {
				ref_counter.decrement(phys_ptr);
			}
		}
	}

	// TODO replace by a `try_clone`?
	/// Clones the mapping for the fork operation. The new mapping is sharing
	/// the same physical memory, for Copy-On-Write.
	///
	/// `vmem` is the virtual memory context for the new mapping
	///
	/// The virtual memory context has to be updated after calling this
	/// function.
	///
	/// The function returns then newly created mapping.
	pub(super) fn fork(&mut self, vmem: Arc<Mutex<Box<dyn VMem>>>) -> AllocResult<Self> {
		let mut new_mapping = Self {
			begin: self.begin,
			size: self.size,
			flags: self.flags,

			vmem,
			residence: self.residence.clone(),
		};
		let mut vmem = self.vmem.lock();
		let nolazy = (new_mapping.get_flags() & super::MAPPING_FLAG_NOLAZY) != 0;
		if nolazy {
			for i in 0..self.size.get() {
				let virt_ptr = unsafe { self.begin.add(i * memory::PAGE_SIZE) };
				unsafe {
					vmem.unmap(virt_ptr)?;
				}
				new_mapping.map(i)?;
			}
		} else {
			let mut ref_counter = super::PHYSICAL_REF_COUNTER.lock();
			for i in 0..self.size.get() {
				if let Some(phys_ptr) = self.get_physical_page(i, &**vmem) {
					if let Err(errno) = ref_counter.increment(phys_ptr) {
						self.fork_fail_clean(&mut ref_counter, i, &**vmem);
						return Err(errno);
					}
				}
			}
		}
		Ok(new_mapping)
	}

	/// Synchronizes the data on the memory mapping back to the filesystem.
	///
	/// The function does nothing if:
	/// - The mapping is not shared
	/// - The mapping is not associated with a file
	/// - The associated file has been removed or cannot be accessed
	///
	/// If the mapping is lock, the function returns [`crate::errno::EBUSY`].
	pub fn fs_sync(&self) -> EResult<()> {
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
		let vmem = self.vmem.lock();
		unsafe {
			vmem::switch(&**vmem, || {
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
		let _ = self.unmap(0..self.size.get());
	}
}
