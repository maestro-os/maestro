/// TODO doc

use core::cmp::Ordering;
use core::ffi::c_void;
use crate::errno::Errno;
use crate::memory::buddy;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::memory;
use crate::util::boxed::Box;
use crate::util::container::binary_tree::BinaryTree;
use crate::util::list::ListNode;
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

	/// The node of the list storing the mappings sharing the same physical memory.
	pub shared_list: ListNode,
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

			shared_list: ListNode::new_single(),
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
		if (self.flags & super::MAPPING_FLAG_WRITE) != 0 && allocated
			&& (self.shared_list.is_single() || self.flags & super::MAPPING_FLAG_SHARED != 0) {
			flags |= vmem::x86::FLAG_WRITE;
		}
		if (self.flags & super::MAPPING_FLAG_USER) != 0 {
			flags |= vmem::x86::FLAG_USER;
		}
		flags
	}

	/// Maps the mapping to the given virtual memory context with the default page. If the mapping
	/// is marked as nolazy, the function allocates physical memory to be mapped.
	pub fn map_default(&self, vmem: &mut Box::<dyn VMem>) -> Result<(), Errno> {
		let nolazy = (self.flags & super::MAPPING_FLAG_NOLAZY) != 0;
		let default_page = get_default_page();
		let flags = self.get_vmem_flags(nolazy);

		for i in 0..self.size {
			let phys_ptr = if nolazy {
				let ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER);
				if let Err(errno) = ptr {
					self.unmap(vmem);
					return Err(errno);
				}
				ptr.unwrap()
			} else {
				default_page
			};
			let virt_ptr = ((self.begin as usize) + (i * memory::PAGE_SIZE)) as *const c_void;
			if let Err(errno) = vmem.map(phys_ptr, virt_ptr, flags) {
				self.unmap(vmem);
				return Err(errno);
			}
		}

		vmem.flush();
		Ok(())
	}

	// TODO Implement COW
	// TODO Force the current vmem to be bound (if not, bind temporarily to zero or copy)
	/// Maps the page at offset `offset` in the mapping to the given virtual memory context. The
	/// function allocates the physical memory to be mapped. If the memory is already mapped with
	/// non-default physical pages, the function does nothing.
	pub fn map(&self, offset: usize, vmem: &mut Box::<dyn VMem>) -> Result<(), Errno> {
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		if let Some(phys_ptr) = vmem.translate(virt_ptr) {
			if phys_ptr != get_default_page() {
				return Ok(());
			}
		}

		let phys_ptr = buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?;
		// TODO Ensure the memory is zero-init
		let flags = self.get_vmem_flags(true);
		if let Err(errno) = vmem.map(phys_ptr, virt_ptr, flags) {
			buddy::free(phys_ptr, 0);
			return Err(errno);
		}
		vmem.flush();
		Ok(())
	}

	/// Unmaps the mapping from the given virtual memory context.
	pub fn unmap(&self, vmem: &mut Box::<dyn VMem>) {
		// TODO

		vmem.flush();
	}

	/// Updates the given virtual memory context `vmem` according to the mapping for the page at
	/// offset `offset`.
	pub fn update_vmem(&self, offset: usize, vmem: &mut Box::<dyn VMem>) {
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		let phys_ptr_result = vmem.translate(virt_ptr);
		if phys_ptr_result.is_none() {
			return;
		}
		let phys_ptr = phys_ptr_result.unwrap();

		let allocated = phys_ptr != get_default_page();
		let flags = self.get_vmem_flags(allocated);
		vmem.map(phys_ptr, virt_ptr, flags).unwrap();
		vmem.flush();
	}

	/// Clones the mapping for the fork operation. The other mapping is sharing the same physical
	/// memory for Copy-On-Write. `container` is the container in which the new mapping is to be
	/// inserted. The virtual memory context has to be updated after calling this function.
	/// The function returns a mutable reference to the newly created mapping.
	pub fn fork<'a>(&mut self, container: &'a mut BinaryTree::<MemMapping>)
		-> Result::<&'a mut Self, Errno> {
		let new_mapping = Self {
			begin: self.begin,
			size: self.size,
			flags: self.flags,

			shared_list: ListNode::new_single(),
		};
		container.insert(new_mapping)?;

		let new_mapping = container.get(self.begin).unwrap();
		new_mapping.shared_list.insert_after(&mut self.shared_list);
		Ok(new_mapping)
	}
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
