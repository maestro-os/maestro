/// TODO doc

use core::cmp::Ordering;
use core::ffi::c_void;
use crate::memory::buddy;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::memory;
use crate::util::boxed::Box;
use crate::util;

/// A pointer to the default physical page of memory. This page is meant to be mapped in read-only
/// and is a placeholder for pages that are accessed without being allocated nor written.
static mut DEFAULT_PAGE: Option::<*const c_void> = None;

/// Returns a pointer to the default physical page.
fn get_default_page() -> *const c_void {
	let default_page = unsafe { // Access to global variable
		&mut DEFAULT_PAGE
	};

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

	// TODO Add sharing informations
}

impl Ord for MemMapping {
	fn cmp(&self, other: &Self) -> Ordering {
		self.begin.cmp(&other.begin)
	}
}

impl Eq for MemMapping {}

impl PartialEq for MemMapping {
	fn eq(&self, other: &Self) -> bool {
		self.begin == other.begin
	}
}

impl PartialOrd for MemMapping {
	fn partial_cmp(&self, other: &Self) -> Option::<Ordering> {
		Some(self.begin.cmp(&other.begin))
	}
}

impl PartialEq::<*const c_void> for MemMapping {
	fn eq(&self, other: &*const c_void) -> bool {
		self.begin == *other
	}
}

impl PartialOrd::<*const c_void> for MemMapping {
	fn partial_cmp(&self, other: &*const c_void) -> Option::<Ordering> {
		Some(self.begin.cmp(other))
	}
}

impl MemMapping {
	/// Creates a new instance.
	/// `begin` is the pointer on the virtual memory to the beginning of the mapping. This pointer
	/// must be page-aligned.
	/// `size` is the size of the mapping in pages. The size must be greater than 0.
	/// `flags` the mapping's flags
	pub fn new(begin: *const c_void, size: usize, flags: u8) -> Self {
		debug_assert!(util::is_aligned(begin, memory::PAGE_SIZE));
		debug_assert!(size > 0);

		Self {
			begin: begin,
			size: size,
			flags: flags,
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

	/// Returns the flags for the virtual memory context mapping.
	fn get_vmem_flags(&self, allocated: bool) -> u32 {
		let mut flags = 0;
		if allocated && (self.flags & super::MAPPING_FLAG_WRITE) != 0 {
			flags |= vmem::x86::FLAG_WRITE;
		}
		if (self.flags & super::MAPPING_FLAG_USER) != 0 {
			flags |= vmem::x86::FLAG_USER;
		}
		flags
	}

	/// Maps the mapping to the given virtual memory context with the default page. If the mapping
	/// is marked as nolazy, the function allocates physical memory to be mapped.
	pub fn map_default(&self, vmem: &mut Box::<dyn VMem>) -> Result::<(), ()> {
		let nolazy = (self.flags & super::MAPPING_FLAG_NOLAZY) != 0;
		let default_page = get_default_page();
		let flags = self.get_vmem_flags(nolazy);

		for i in 0..self.size {
			let phys_ptr = if nolazy {
				let ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER);
				if ptr.is_err() {
					self.unmap(vmem);
					return Err(());
				}
				ptr.unwrap()
			} else {
				default_page
			};
			let virt_ptr = ((self.begin as usize) + (i * memory::PAGE_SIZE)) as *const c_void;
			if vmem.map(phys_ptr, virt_ptr, flags).is_err() {
				self.unmap(vmem);
				return Err(());
			}
		}
		Ok(())
	}

	/// Maps the page at offset `offset` in the mapping to the given virtual memory context. The
	/// function allocates the physical memory to be mapped. If the memory is already mapped with
	/// non-default physical pages, the function does nothing.
	pub fn map(&self, _offset: usize, _vmem: &mut Box::<dyn VMem>) -> Result::<(), ()> {
		// TODO
		Ok(())
	}

	/// Unmaps the mapping from the given virtual memory context.
	pub fn unmap(&self, _vmem: &mut Box::<dyn VMem>) {
		// TODO
	}

	// TODO
}
