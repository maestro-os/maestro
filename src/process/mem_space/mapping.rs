//! A memory mapping is a region of virtual memory that a process can access. It may be mapped
//! at the process's creation or by the process itself using system calls.

use core::cmp::Ordering;
use core::ffi::c_void;
use core::ptr::NonNull;
use core::ptr;
use crate::errno::Errno;
use crate::memory::buddy;
use crate::memory::vmem::VMem;
use crate::memory::vmem::vmem_switch;
use crate::memory::vmem;
use crate::memory;
use crate::util::boxed::Box;
use crate::util::lock::mutex::*;
use crate::util;
use super::MemSpace;

/// A pointer to the default physical page of memory. This page is meant to be mapped in read-only
/// and is a placeholder for pages that are accessed without being allocated nor written.
static mut DEFAULT_PAGE: Mutex::<Option::<*const c_void>> = Mutex::new(None);

/// Returns a pointer to the default physical page.
fn get_default_page() -> *const c_void {
	let m = unsafe { // Safe because using a Mutex
		&mut DEFAULT_PAGE
	};
	let mut guard = MutexGuard::new(m);
	let default_page = guard.get_mut();

	if default_page.is_none() {
		let ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_KERNEL);
		if let Ok(ptr) = ptr {
			*default_page = Some(ptr);
		} else {
			kernel_panic!("Cannot allocate default memory page!", 0);
		}
	}

	default_page.unwrap()
}

/// A mapping in the memory space.
pub struct MemMapping {
	/// Pointer on the virtual memory to the beginning of the mapping
	begin: *const c_void,
	/// The size of the mapping in pages.
	size: usize,
	/// The mapping's flags.
	flags: u8,

	/// Pointer to the virtual memory context handler.
	vmem: NonNull::<dyn VMem>, // TODO Use a weak pointer
}

impl MemMapping {
	/// Creates a new instance.
	/// `begin` is the pointer on the virtual memory to the beginning of the mapping. This pointer
	/// must be page-aligned.
	/// `size` is the size of the mapping in pages. The size must be greater than 0.
	/// `flags` the mapping's flags
	/// `vmem` is the virtual memory context handler.
	pub fn new(begin: *const c_void, size: usize, flags: u8, vmem: NonNull::<dyn VMem>) -> Self {
		debug_assert!(util::is_aligned(begin, memory::PAGE_SIZE));
		debug_assert!(size > 0);

		Self {
			begin,
			size,
			flags,

			vmem,
		}
	}

	/// Returns a pointer on the virtual memory to the beginning of the mapping.
	pub fn get_begin(&self) -> *const c_void {
		self.begin
	}

	/// Returns the size of the mapping in memory pages.
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Returns the mapping's flags.
	pub fn get_flags(&self) -> u8 {
		self.flags
	}

	/// Returns a reference to the virtual memory context handler associated with the mapping.
	pub fn get_vmem(&self) -> &'static dyn VMem {
		unsafe {
			&*self.vmem.as_ptr()
		}
	}

	/// Returns a mutable reference to the virtual memory context handler associated with the
	/// mapping.
	pub fn get_mut_vmem(&mut self) -> &'static mut dyn VMem {
		unsafe {
			&mut *self.vmem.as_ptr()
		}
	}

	/// Tells whether the mapping contains the given virtual address `ptr`.
	pub fn contains_ptr(&self, ptr: *const c_void) -> bool {
		ptr >= self.begin && ptr < (self.begin as usize + self.size * memory::PAGE_SIZE) as _
	}

	/// Returns a pointer to the physical page of memory associated with the mapping at page offset
	/// `offset`. If no page is associated, the function returns None.
	pub fn get_physical_page(&self, offset: usize) -> Option::<*const c_void> {
		let vmem = self.get_vmem();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		let phys_ptr = vmem.translate(virt_ptr)?;
		if phys_ptr != get_default_page() {
			Some(phys_ptr)
		} else {
			None
		}
	}

	/// Tells whether the page at offset `offset` in the mapping is shared or not.
	pub fn is_shared(&self, offset: usize) -> bool {
		if let Some(phys_ptr) = self.get_physical_page(offset) {
			unsafe { // Safe because the global variable is wrapped into a Mutex
				let ref_counter = super::PHYSICAL_REF_COUNTER.lock();
				ref_counter.get().is_shared(phys_ptr)
			}
		} else {
			false
		}
	}

	/// Tells whether the page at offset `offset` is waiting for Copy-On-Write.
	pub fn is_cow(&self, offset: usize) -> bool {
		(self.flags & super::MAPPING_FLAG_SHARED) == 0 && self.is_shared(offset)
	}

	/// Returns the flags for the virtual memory context for the given virtual page offset.
	/// `allocated` tells whether the page has been physically allocated.
	/// `offset` is the offset of the page in the mapping.
	fn get_vmem_flags(&self, allocated: bool, offset: usize) -> u32 {
		let mut flags = 0;
		if (self.flags & super::MAPPING_FLAG_WRITE) != 0 && allocated && !self.is_cow(offset) {
			flags |= vmem::x86::FLAG_WRITE;
		}
		if (self.flags & super::MAPPING_FLAG_USER) != 0 {
			flags |= vmem::x86::FLAG_USER;
		}
		flags
	}

	/// Maps the mapping to the given virtual memory context with the default page. If the mapping
	/// is marked as nolazy, the function allocates physical memory and maps it instead of the
	/// default page.
	pub fn map_default(&mut self) -> Result<(), Errno> {
		let vmem = self.get_mut_vmem();
		let nolazy = (self.flags & super::MAPPING_FLAG_NOLAZY) != 0;
		let default_page = get_default_page();

		for i in 0..self.size {
			let phys_ptr = {
				if nolazy {
					let ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER);
					if let Err(errno) = ptr {
						self.unmap();
						return Err(errno);
					}
					ptr.unwrap()
				} else {
					default_page
				}
			};
			let virt_ptr = ((self.begin as usize) + (i * memory::PAGE_SIZE)) as *const c_void;
			let flags = self.get_vmem_flags(nolazy, i);

			if let Err(errno) = vmem.map(phys_ptr, virt_ptr, flags) {
				self.unmap();
				return Err(errno);
			}
		}

		vmem.flush();
		Ok(())
	}

	/// Maps the page at offset `offset` in the mapping to the given virtual memory context. The
	/// function allocates the physical memory to be mapped.
	/// If the mapping is in forking state, the function shall apply Copy-On-Write and allocate
	/// a new physical page with the same data.
	pub fn map(&mut self, offset: usize) -> Result<(), Errno> {
		let vmem = self.get_mut_vmem();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *mut c_void;
		let cow = self.is_cow(offset);
		let cow_buffer = {
			if cow {
				let mut cow_buffer = Box::<[u8; memory::PAGE_SIZE]>::new([0; memory::PAGE_SIZE])?;
				unsafe {
					ptr::copy_nonoverlapping(virt_ptr, cow_buffer.as_mut_ptr() as _,
						memory::PAGE_SIZE);
				}

				Some(cow_buffer)
			} else {
				None
			}
		};

		let prev_phys_ptr = self.get_physical_page(offset);
		if !cow && prev_phys_ptr.is_some() {
			return Ok(());
		}

		let new_phys_ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?;
		let flags = self.get_vmem_flags(true, offset);

		{
			let mut ref_counter = unsafe {
				super::PHYSICAL_REF_COUNTER.lock()
			};
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

		vmem_switch(vmem, || {
			unsafe {
				vmem::write_lock_wrap(|| {
					if let Some(buffer) = &cow_buffer {
						ptr::copy_nonoverlapping(buffer.as_ptr() as *const c_void,
							virt_ptr as *mut c_void, memory::PAGE_SIZE);
					} else {
						util::bzero(virt_ptr as _, memory::PAGE_SIZE);
					}
				});
			}
		});
		vmem.flush(); // TODO Remove?

		Ok(())
	}

	/// Unmaps the mapping from the given virtual memory context.
	pub fn unmap(&self) {
		// TODO
		// TODO Remove the physical memory only if the page is not shared
		todo!();

		// self.get_vmem().flush();
	}

	/// Updates the virtual memory context according to the mapping for the page at offset
	/// `offset`.
	pub fn update_vmem(&mut self, offset: usize) {
		let vmem = self.get_mut_vmem();
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;

		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			let allocated = phys_ptr != get_default_page();
			let flags = self.get_vmem_flags(allocated, offset);
			// Cannot fail because the page is already mapped
			vmem.map(phys_ptr, virt_ptr, flags).unwrap();

			// TODO Use page invalidation instead if available
			vmem.flush();
		}
	}

	/// Clones the mapping for the fork operation. The other mapping is sharing the same physical
	/// memory for Copy-On-Write.
	/// `container` is the container in which the new mapping is to be inserted.
	/// The virtual memory context has to be updated after calling this function.
	/// The function returns a mutable reference to the newly created mapping.
	pub fn fork<'a>(&mut self, mem_space: &'a mut MemSpace) -> Result<&'a mut Self, Errno> {
		let new_mapping = Self {
			begin: self.begin,
			size: self.size,
			flags: self.flags,

			vmem: NonNull::new(mem_space.get_vmem().as_mut()).unwrap(),
		};

		unsafe { // Safe because the global variable is wrapped into a Mutex
			let mut ref_counter_lock = super::PHYSICAL_REF_COUNTER.lock();
			let ref_counter = ref_counter_lock.get_mut();

			for i in 0..self.size {
				if let Some(phys_ptr) = self.get_physical_page(i) {
					if let Err(errno) = ref_counter.increment(phys_ptr) {
						// TODO Clean
						for j in 0..i {
							if let Some(phys_ptr) = self.get_physical_page(j) {
								ref_counter.decrement(phys_ptr);
							}
						}

						return Err(errno);
					}
				}
			}
		};

		mem_space.mappings.insert(new_mapping)?;
		Ok(mem_space.mappings.get_mut(self.begin).unwrap())
	}
}

impl Ord for MemMapping {
	fn cmp(&self, other: &Self) -> Ordering {
		(self.begin as usize).cmp(&(other.begin as usize))
	}
}

impl Eq for MemMapping {}

impl PartialEq for MemMapping {
	fn eq(&self, other: &Self) -> bool {
		ptr::eq(self.begin, other.begin)
	}
}

impl PartialOrd for MemMapping {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some((self.begin as usize).cmp(&(other.begin as usize)))
	}
}

impl PartialEq<*const c_void> for MemMapping {
	fn eq(&self, other: &*const c_void) -> bool {
		ptr::eq(self.begin, *other)
	}
}

impl PartialOrd<*const c_void> for MemMapping {
	fn partial_cmp(&self, other: &*const c_void) -> Option<Ordering> {
		Some((self.begin as usize).cmp(&((*other) as usize)))
	}
}

impl Drop for MemMapping {
	fn drop(&mut self) {
		self.unmap();
	}
}
