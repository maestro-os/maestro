//! A memory mapping is a region of virtual memory that a process can access. It
//! may be mapped at the process's creation or by the process itself using
//! system calls.

use core::ffi::c_void;
use core::fmt;
use core::ptr::NonNull;
use core::ptr;
use core::slice;
use crate::errno::Errno;
use crate::file::open_file::OpenFile;
use crate::memory::buddy;
use crate::memory::malloc;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::memory;
use crate::process::mem_space::physical_ref_counter::PhysRefCounter;
use crate::process::oom;
use crate::util::io::IO;
use crate::util::lock::*;
use crate::util::ptr::SharedPtr;
use crate::util;
use super::MemSpace;
use super::gap::MemGap;

/// A pointer to the default physical page of memory. This page is meant to be
/// mapped in read-only and is a placeholder for pages that are accessed without
/// being allocated nor written.
static DEFAULT_PAGE: Mutex<Option<*const c_void>> = Mutex::new(None);

/// Returns a pointer to the default physical page.
fn get_default_page() -> *const c_void {
	let guard = DEFAULT_PAGE.lock();
	let default_page = guard.get_mut();

	match default_page {
		Some(ptr) => *ptr,

		// Lazy allocation
		None => {
			let Ok(ptr) = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_KERNEL) else {
				kernel_panic!("Cannot allocate default memory page!");
			};

			*default_page = Some(ptr);
			ptr
		},
	}
}

// TODO update the number of reference to the open file when necessary

// TODO Disallow clone and use a special function + Drop to increment/decrement reference counters
/// Enumeration of map residences.
/// A map residence is where a memory mapping may be backed, for both saving memory and staying in
/// sync with storage.
///
/// A mapping may be switched from `MainMemory` to `Swap` and vice-versa.
#[derive(Clone, Debug)]
enum MapResidence {
	/// The mapping does not reside anywhere except on the main memory.
	None,

	/// The mapping resides in a file.
	File {
		/// The location of the file.
		file: SharedPtr<OpenFile>,

		/// The offset of the mapping in the file.
		off: u64,
	},

	/// The mapping resides in swap space.
	Swap {
		/// The location of the swap space.
		swap_file: SharedPtr<OpenFile>,

		/// The ID of the slot occupied by the mapping.
		slot_id: u32,
		/// The page offset in the slot.
		page_off: usize,
	},
}

impl MapResidence {
	/// Adds a value of `pages` pages to the offset of the residence, if applicable.
	pub fn offset_add(&mut self, pages: usize) {
		match self {
			Self::File { off, ..  } => *off += pages as u64 * memory::PAGE_SIZE as u64,

			Self::Swap { page_off, ..  } => *page_off += pages,

			_ => {},
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
	size: usize,
	/// The mapping's flags.
	flags: u8,

	/// The residence of the mapping.
	residence: MapResidence,

	/// Pointer to the virtual memory context handler.
	vmem: NonNull<dyn VMem>,
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
	/// If None, the mapping doesn't point to any file.
	/// - `vmem` is the virtual memory context handler associated with the mapping.
	pub fn new(
		begin: *const c_void,
		size: usize,
		flags: u8,
		file: Option<(SharedPtr<OpenFile>, u64)>,
		vmem: NonNull<dyn VMem>,
	) -> Self {
		debug_assert!(util::is_aligned(begin, memory::PAGE_SIZE));
		debug_assert!(size > 0);

		let residence = match file {
			Some((file, off)) => MapResidence::File {
				file,

				off,
			},

			None => MapResidence::None,
		};

		Self {
			begin,
			size,
			flags,

			residence,

			vmem,
		}
	}

	/// Returns a pointer on the virtual memory to the beginning of the mapping.
	#[inline(always)]
	pub fn get_begin(&self) -> *const c_void {
		self.begin
	}

	/// Returns the size of the mapping in memory pages.
	#[inline(always)]
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Returns the mapping's flags.
	#[inline(always)]
	pub fn get_flags(&self) -> u8 {
		self.flags
	}

	/// Returns a reference to the virtual memory context handler associated
	/// with the mapping.
	#[inline(always)]
	pub fn get_vmem(&self) -> &'static dyn VMem {
		unsafe { &*self.vmem.as_ptr() }
	}

	/// Returns a mutable reference to the virtual memory context handler
	/// associated with the mapping.
	#[inline(always)]
	pub fn get_mut_vmem(&mut self) -> &'static mut dyn VMem {
		unsafe { &mut *self.vmem.as_ptr() }
	}

	/// Tells whether the mapping contains the given virtual address `ptr`.
	#[inline(always)]
	pub fn contains_ptr(&self, ptr: *const c_void) -> bool {
		ptr >= self.begin && ptr < (self.begin as usize + self.size * memory::PAGE_SIZE) as _
	}

	/// Returns a pointer to the physical page of memory associated with the
	/// mapping at page offset `offset`.
	///
	/// If no page is associated, the function returns None.
	pub fn get_physical_page(&self, offset: usize) -> Option<*const c_void> {
		let vmem = self.get_vmem();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		let phys_ptr = vmem.translate(virt_ptr)?;

		if phys_ptr != get_default_page() {
			Some(phys_ptr)
		} else {
			None
		}
	}

	/// Tells whether the page at offset `offset` in the mapping is shared or
	/// not.
	pub fn is_shared(&self, offset: usize) -> bool {
		if let Some(phys_ptr) = self.get_physical_page(offset) {
			let ref_counter = super::PHYSICAL_REF_COUNTER.lock();
			ref_counter.get_mut().is_shared(phys_ptr)
		} else {
			false
		}
	}

	/// Tells whether the page at offset `offset` is waiting for Copy-On-Write.
	pub fn is_cow(&self, offset: usize) -> bool {
		self.flags & super::MAPPING_FLAG_SHARED == 0 && self.is_shared(offset)
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

	/// Maps the mapping to the given virtual memory context with the default page.
	///
	/// If the mapping is marked as nolazy, the function allocates physical memory and maps it
	/// instead of the default page.
	pub fn map_default(&mut self) -> Result<(), Errno> {
		let vmem = self.get_mut_vmem();
		let nolazy = (self.flags & super::MAPPING_FLAG_NOLAZY) != 0;
		let default_page = get_default_page();

		for i in 0..self.size {
			let phys_ptr = {
				if nolazy {
					match buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER) {
						Ok(ptr) => ptr,
						Err(e) => {
							self.unmap()?;
							return Err(e);
						}
					}
				} else {
					default_page
				}
			};
			let virt_ptr = ((self.begin as usize) + (i * memory::PAGE_SIZE)) as *const c_void;
			let flags = self.get_vmem_flags(nolazy, i);

			if let Err(errno) = vmem.map(phys_ptr, virt_ptr, flags) {
				if nolazy {
					buddy::free(phys_ptr, 0);
				}
				self.unmap()?;

				return Err(errno);
			}
		}

		Ok(())
	}

	// TODO Add support for file descriptors
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
		let vmem = self.get_mut_vmem();
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
		if cow_buffer.is_none() && prev_phys_ptr.is_some() {
			return Ok(());
		}

		let new_phys_ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?;
		let flags = self.get_vmem_flags(true, offset);

		{
			let ref_counter = super::PHYSICAL_REF_COUNTER.lock();
			if let Err(errno) = ref_counter.get_mut().increment(new_phys_ptr) {
				buddy::free(new_phys_ptr, 0);
				return Err(errno);
			}
			if let Err(errno) = vmem.map(new_phys_ptr, virt_ptr, flags) {
				ref_counter.get_mut().decrement(new_phys_ptr);
				buddy::free(new_phys_ptr, 0);
				return Err(errno);
			}

			if let Some(prev_phys_ptr) = prev_phys_ptr {
				ref_counter.get_mut().decrement(prev_phys_ptr);
			}
		}

		// Copying data if necessary
		unsafe {
			vmem::switch(vmem, move || {
				vmem::write_lock_wrap(|| {
					if let Some(buffer) = cow_buffer {
						ptr::copy_nonoverlapping(
							buffer.as_ptr() as *const c_void,
							virt_ptr as *mut c_void,
							memory::PAGE_SIZE,
						);
					} else {
						util::bzero(virt_ptr, memory::PAGE_SIZE);
					}
				});
			});
		}

		Ok(())
	}

	/// Frees the physical page at offset `offset` of the mapping.
	///
	/// If the page is shared, it is not freed but the reference counter is decreased.
	fn free_phys_page(&mut self, offset: usize) {
		let vmem = self.get_mut_vmem();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;

		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			if phys_ptr == get_default_page() {
				return;
			}

			let can_free = {
				let ref_counter = super::PHYSICAL_REF_COUNTER.lock();

				ref_counter.get_mut().decrement(phys_ptr);
				ref_counter.get().can_free(phys_ptr)
			};

			if can_free {
				buddy::free(phys_ptr, 0);
			}
		}
	}

	/// Unmaps the mapping from the given virtual memory context.
	///
	/// If the physical pages the mapping points to are not shared, the function frees them.
	///
	/// This function doesn't flush the virtual memory context.
	pub fn unmap(&mut self) -> Result<(), Errno> {
		// Removing physical pages
		for i in 0..self.size {
			self.free_phys_page(i);
		}

		// Unmapping physical pages
		let vmem = self.get_mut_vmem();
		oom::wrap(|| vmem.unmap_range(self.begin, self.size));

		Ok(())
	}

	/// Partially unmaps the current mapping, creating up to two new mappings and one gap.
	///
	/// Arguments:
	/// - `begin` is the index of the first page to be unmapped.
	/// - `size` is the number of pages to unmap.
	///
	/// If the region to be unmapped is out of bounds, it is truncated to the end of the mapping.
	/// The newly created mappings correspond to the remaining pages.
	/// The newly created gap is in place of the unmapped portion.
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
		let prev = {
			if begin > 0 {
				Some(Self {
					begin: self.begin,
					size: begin,
					flags: self.flags,

					residence: self.residence.clone(),

					vmem: self.vmem,
				})
			} else {
				None
			}
		};

		let gap = {
			if size > 0 {
				Some(MemGap::new(begin_ptr, size))
			} else {
				None
			}
		};

		// The mapping located after the gap to be created
		let next = {
			let end = begin + size;

			if end < self.size {
				let map_begin = unsafe { self.begin.add(end * memory::PAGE_SIZE) };
				let map_size = self.size - end;

				let mut residence = self.residence.clone();
				residence.offset_add(end);

				Some(Self {
					begin: map_begin,
					size: map_size,
					flags: self.flags,

					residence,

					vmem: self.vmem,
				})
			} else {
				None
			}
		};

		// Freeing pages that will be replaced by the gap
		for i in begin..(begin + size) {
			self.free_phys_page(i);
		}

		// Unmapping physical pages
		let vmem = self.get_mut_vmem();
		oom::wrap(|| vmem.unmap_range(begin_ptr, size));

		(prev, gap, next)
	}

	/// Updates the virtual memory context according to the mapping for the page
	/// at offset `offset`.
	pub fn update_vmem(&mut self, offset: usize) {
		let vmem = self.get_mut_vmem();
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
			for i in 0..self.size {
				let virt_ptr = (self.begin as usize + i * memory::PAGE_SIZE) as *const c_void;
				new_mapping.get_mut_vmem().unmap(virt_ptr)?;
				new_mapping.map(i)?;
			}
		} else {
			let ref_counter_guard = super::PHYSICAL_REF_COUNTER.lock();
			let ref_counter = ref_counter_guard.get_mut();

			for i in 0..self.size {
				if let Some(phys_ptr) = self.get_physical_page(i) {
					if let Err(errno) = ref_counter.increment(phys_ptr) {
						self.fork_fail_clean(ref_counter, i);
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
				let file_guard = file.lock();
				let file = file_guard.get_mut();

				// TODO Make use of dirty flag if present on the current architecure to update
				// only pages that have been modified
				let slice = slice::from_raw_parts(
					self.begin as *mut u8,
					self.size * memory::PAGE_SIZE
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
		let end = unsafe { self.begin.add(self.size * memory::PAGE_SIZE) };

		write!(
			f,
			"begin: {:p}; end: {:p}; flags: {}; residence: {:?}",
			self.begin,
			end,
			self.flags,
			self.residence,
		)
	}
}
