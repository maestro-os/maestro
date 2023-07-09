//! A memory mapping is a region of virtual memory that a process can access. It
//! may be mapped at the process's creation or by the process itself using
//! system calls.

use super::gap::MemGap;
use super::MapResidence;
use super::MemSpace;
use crate::errno::Errno;
use crate::memory;
use crate::memory::buddy;
use crate::memory::malloc;
use crate::memory::physical_ref_counter::PhysRefCounter;
use crate::memory::vmem;
use crate::memory::vmem::VMem;
use crate::process::oom;
use crate::util::io::IO;
use crate::util::lock::*;
use core::ffi::c_void;
use core::fmt;
use core::num::NonZeroUsize;
use core::ptr;
use core::ptr::NonNull;
use core::slice;

/// A pointer to the default physical page of memory.
///
/// This page is meant to be mapped in read-only and is a placeholder for pages that are accessed
/// without being allocated nor written.
static DEFAULT_PAGE: Mutex<Option<*const c_void>> = Mutex::new(None);

/// Returns a physical pointer to the default page.
fn get_default_page() -> *const c_void {
	let mut default_page = DEFAULT_PAGE.lock();

	match &mut *default_page {
		Some(ptr) => *ptr,

		// Lazy allocation
		None => {
			let Ok(ptr) = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_KERNEL) else {
				kernel_panic!("Cannot allocate default memory page!");
			};

			// Zero page
			let virt_ptr = memory::kern_to_virt(ptr) as *mut u8;
			let slice = unsafe { slice::from_raw_parts_mut(virt_ptr, memory::PAGE_SIZE) };
			slice.fill(0);

			*default_page = Some(ptr);
			ptr
		}
	}
}

/// A mapping in the memory space.
///
/// **Warning**: When dropped, mappings do not unmap themselves. It is the
/// caller's responsibility to call `unmap` or `partial_unmap` before dropping a
/// mapping. Failure to do so may result in a memory leak.
#[derive(Clone)]
pub struct MemMapping {
	/// Pointer on the virtual memory to the beginning of the mapping
	begin: *const c_void,
	/// The size of the mapping in pages.
	size: NonZeroUsize,
	/// The mapping's flags.
	flags: u8,

	/// The residence of the mapping.
	residence: MapResidence,

	/// Pointer to the virtual memory context handler.
	vmem: NonNull<dyn VMem>, // TODO replace by a safer shared container
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
	/// - `vmem` is the virtual memory context handler associated with the mapping.
	pub fn new(
		begin: *const c_void,
		size: NonZeroUsize,
		flags: u8,
		residence: MapResidence,
		vmem: NonNull<dyn VMem>,
	) -> Self {
		debug_assert!(begin.is_aligned_to(memory::PAGE_SIZE));

		Self {
			begin,
			size,
			flags,

			residence,

			vmem,
		}
	}

	/// Returns a pointer on the virtual memory to the beginning of the mapping.
	pub fn get_begin(&self) -> *const c_void {
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
	pub fn get_vmem(&self) -> &mut dyn VMem {
		unsafe { &mut *self.vmem.as_ptr() }
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
	pub fn get_physical_page(&self, offset: usize) -> Option<*const c_void> {
		if offset >= self.size.get() {
			return None;
		}

		let vmem = self.get_vmem();
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
	pub fn is_shared(&self, offset: usize) -> bool {
		if let Some(phys_ptr) = self.get_physical_page(offset) {
			let ref_counter = super::PHYSICAL_REF_COUNTER.lock();
			ref_counter.is_shared(phys_ptr)
		} else {
			false
		}
	}

	/// Tells whether the page at offset `offset` is waiting for Copy-On-Write.
	pub fn is_cow(&self, offset: usize) -> bool {
		self.flags & super::MAPPING_FLAG_SHARED == 0
			&& self.residence.is_normal()
			&& self.is_shared(offset)
	}

	// TODO Move into architecture-specific code
	/// Returns the flags for the virtual memory context for the given virtual page offset.
	///
	/// Arguments:
	/// - `allocated` tells whether the page has been physically allocated.
	/// - `offset` is the offset of the page in the mapping.
	fn get_vmem_flags(&self, allocated: bool, offset: usize) -> u32 {
		let mut flags = 0;

		if self.flags & super::MAPPING_FLAG_WRITE != 0 && allocated && !self.is_cow(offset) {
			flags |= vmem::x86::FLAG_WRITE;
		}
		if self.flags & super::MAPPING_FLAG_USER != 0 {
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
	pub fn map(&mut self, offset: usize) -> Result<(), Errno> {
		let vmem = self.get_vmem();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *mut c_void;

		let cow_buffer = {
			if self.is_cow(offset) {
				let mut cow_buffer = malloc::Alloc::<u8>::new_default(memory::PAGE_SIZE)?;

				unsafe {
					ptr::copy_nonoverlapping(
						virt_ptr,
						cow_buffer.as_ptr_mut() as _,
						memory::PAGE_SIZE,
					);
				}

				Some(cow_buffer)
			} else {
				None
			}
		};

		let prev_phys_ptr = self.get_physical_page(offset);
		if self.residence.is_normal() && cow_buffer.is_none() && prev_phys_ptr.is_some() {
			return Ok(());
		}

		// Map new page
		let new_phys_ptr = self.residence.alloc_page(offset)?;
		let flags = self.get_vmem_flags(true, offset);
		if let Err(errno) = vmem.map(new_phys_ptr, virt_ptr, flags) {
			self.residence.free_page(offset, new_phys_ptr);
			return Err(errno);
		}

		// Free previous page
		if let Some(prev_phys_ptr) = prev_phys_ptr {
			self.residence.free_page(offset, prev_phys_ptr);
		}

		// Copying data if necessary
		if self.residence.is_normal() {
			unsafe {
				// FIXME: switching vmem at each call to `map` is suboptimal (try to batch)
				vmem::switch(vmem, move || {
					vmem::write_lock_wrap(|| {
						if let Some(buffer) = cow_buffer {
							ptr::copy_nonoverlapping(
								buffer.as_ptr() as *const c_void,
								virt_ptr as *mut c_void,
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
	/// If the mapping is marked as nolazy, the function allocates physical memory and maps it
	/// instead of the default page.
	///
	/// The default page is dependent on the nature of the mapping's residence.
	pub fn map_default(&mut self) -> Result<(), Errno> {
		let use_default =
			self.flags & super::MAPPING_FLAG_NOLAZY == 0 && self.residence.is_normal();

		if use_default {
			let vmem = self.get_vmem();
			let default_page = get_default_page();

			for i in 0..self.size.get() {
				let virt_ptr = unsafe { self.begin.add(i * memory::PAGE_SIZE) };
				let flags = self.get_vmem_flags(false, i);

				vmem.map(default_page, virt_ptr, flags)?;
			}
		} else {
			for i in 0..self.size.get() {
				if let Err(errno) = self.map(i) {
					self.unmap()?;
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
		let vmem = self.get_vmem();
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
	pub fn unmap(&mut self) -> Result<(), Errno> {
		// Removing physical pages
		for i in 0..self.size.get() {
			self.free_phys_page(i);
		}

		// Unmapping physical pages
		let vmem = self.get_vmem();
		oom::wrap(|| vmem.unmap_range(self.begin, self.size.get()));

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
	/// If the mapping is totaly unmapped, the function returns no new mappings.
	///
	/// The function doesn't flush the virtual memory context.
	pub fn partial_unmap(
		mut self,
		begin: usize,
		size: usize,
	) -> (Option<Self>, Option<MemGap>, Option<Self>) {
		let begin_ptr = unsafe { self.begin.add(begin * memory::PAGE_SIZE) };

		// The mapping located before the gap to be created
		let prev = NonZeroUsize::new(begin).map(|begin| Self {
			begin: self.begin,
			size: begin,
			flags: self.flags,

			residence: self.residence.clone(),

			vmem: self.vmem,
		});

		let gap = NonZeroUsize::new(size).map(|size| MemGap::new(begin_ptr, size));

		// The mapping located after the gap to be created
		let next = {
			let end = begin + size;

			self.size
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

						vmem: self.vmem,
					}
				})
		};

		// Freeing pages that will be replaced by the gap
		for i in begin..(begin + size) {
			self.free_phys_page(i);
		}

		// Unmapping physical pages
		let vmem = self.get_vmem();
		oom::wrap(|| vmem.unmap_range(begin_ptr, size));

		(prev, gap, next)
	}

	/// Updates the virtual memory context according to the mapping for the page
	/// at offset `offset`.
	pub fn update_vmem(&mut self, offset: usize) {
		let vmem = self.get_vmem();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;

		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			let allocated = phys_ptr != get_default_page();
			let flags = self.get_vmem_flags(allocated, offset);
			// Cannot fail because the page for the vmem structure is already mapped
			vmem.map(phys_ptr, virt_ptr, flags).unwrap();
		}
	}

	/// After a fork operation failed, frees the pages that were already
	/// allocated.
	///
	/// `n` is the number of pages to free from the beginning.
	fn fork_fail_clean(&self, ref_counter: &mut PhysRefCounter, n: usize) {
		for i in 0..n {
			if let Some(phys_ptr) = self.get_physical_page(i) {
				ref_counter.decrement(phys_ptr);
			}
		}
	}

	/// Clones the mapping for the fork operation. The other mapping is sharing
	/// the same physical memory for Copy-On-Write.
	///
	/// `container` is the container in which the new mapping is to be inserted.
	///
	/// The virtual memory context has to be updated after calling this
	/// function.
	///
	/// The function returns a mutable reference to the newly created mapping.
	pub fn fork<'a>(&mut self, mem_space: &'a mut MemSpace) -> Result<&'a mut Self, Errno> {
		let mut new_mapping = Self {
			begin: self.begin,
			size: self.size,
			flags: self.flags,

			residence: self.residence.clone(),

			vmem: NonNull::new(mem_space.get_vmem().as_mut()).unwrap(),
		};
		let nolazy = (new_mapping.get_flags() & super::MAPPING_FLAG_NOLAZY) != 0;

		if nolazy {
			for i in 0..self.size.get() {
				let virt_ptr = unsafe { self.begin.add(i * memory::PAGE_SIZE) };

				new_mapping.get_vmem().unmap(virt_ptr)?;
				new_mapping.map(i)?;
			}
		} else {
			let mut ref_counter = super::PHYSICAL_REF_COUNTER.lock();

			for i in 0..self.size.get() {
				if let Some(phys_ptr) = self.get_physical_page(i) {
					if let Err(errno) = ref_counter.increment(phys_ptr) {
						self.fork_fail_clean(&mut ref_counter, i);
						return Err(errno);
					}
				}
			}
		}

		mem_space
			.mappings
			.insert(new_mapping.get_begin(), new_mapping)
	}

	/// Synchronizes the data on the memory mapping back to the filesystem.
	///
	/// The function does nothing if the mapping is not shared or not associated with a file.
	pub fn fs_sync(&mut self) -> Result<(), Errno> {
		if self.flags & super::MAPPING_FLAG_SHARED == 0 {
			return Ok(());
		}
		let MapResidence::File {
			file,
			off,
		} = &self.residence else {
			return Ok(());
		};

		unsafe {
			vmem::switch(self.get_vmem(), || {
				let mut file = file.lock();

				// TODO Make use of dirty flag if present on the current architecure to update
				// only pages that have been modified
				let slice = slice::from_raw_parts(
					self.begin as *mut u8,
					self.size.get() * memory::PAGE_SIZE,
				);

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
