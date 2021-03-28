/// TODO doc

use core::cmp::Ordering;
use core::ffi::c_void;
use core::mem::ManuallyDrop;
use core::ptr;
use crate::errno::Errno;
use crate::memory::buddy;
use crate::memory::stack::stack_switch;
use crate::memory::vmem::VMem;
use crate::memory::vmem::vmem_switch;
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

	/// Returns the mapping's flags.
	pub fn get_flags(&self) -> u8 {
		self.flags
	}

	/// Tells whether the mapping is in forking state or not. If true, the physical memory is
	/// shared with at least another mapping.
	pub fn is_forking(&self) -> bool {
		!self.shared_list.is_single() && self.flags & super::MAPPING_FLAG_SHARED == 0
	}

	/// Returns the flags for the virtual memory context mapping.
	fn get_vmem_flags(&self, allocated: bool) -> u32 {
		let mut flags = 0;
		if (self.flags & super::MAPPING_FLAG_WRITE) != 0 && allocated && !self.is_forking() {
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

	/// Returns a pointer to the physical page of memory associated with the mapping at page offset
	/// `offset`. `vmem` is the virtual memory context associated with the mapping. If no page is
	/// associated, the function returns None.
	pub fn get_physical_page(&self, offset: usize, vmem: &mut Box::<dyn VMem>)
		-> Option::<*const c_void> {
		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *const c_void;
		let phys_ptr = vmem.translate(virt_ptr)?;
		if phys_ptr != get_default_page() {
			Some(phys_ptr)
		} else {
			None
		}
	}

	/// Tells whether the mapping contains the given virtual address `ptr`.
	pub fn contains_ptr(&self, ptr: *const c_void) -> bool {
		ptr >= self.begin && ptr < (self.begin as usize + self.size * memory::PAGE_SIZE) as _
	}

	/// Returns a random mapping that shares the same physical page.
	fn get_shared_adjacent(&self) -> &'static mut Self {
		let next = self.shared_list.get_next();
		let list_node = {
			if let Some(next) = next {
				next
			} else {
				self.shared_list.get_prev().unwrap()
			}
		};
		let inner_offset = crate::offset_of!(Self, shared_list);
		list_node.get_mut::<Self>(inner_offset)
	}

	// TODO Use RAII to free allocated memory on error?
	/// Maps the page at offset `offset` in the mapping to the given virtual memory context. The
	/// function allocates the physical memory to be mapped.
	/// If the mapping is in forking state, the function shall apply Copy-On-Write and allocate
	/// a new physical page with the same data.
	pub fn map(&mut self, offset: usize, vmem: &mut Box::<dyn VMem>) -> Result<(), Errno> {
		let mut tmp_stack = Box::<[u8; memory::PAGE_SIZE]>::new([0; memory::PAGE_SIZE])?;

		let virt_ptr = (self.begin as usize + offset * memory::PAGE_SIZE) as *mut _;
		let cow = self.is_forking();
		let cow_buffer = {
			if cow {
				let cow_buffer = Box::<[u8; memory::PAGE_SIZE]>::new([0; memory::PAGE_SIZE])?;
				unsafe { // Call to unsafe function
					ptr::copy_nonoverlapping(virt_ptr,
						cow_buffer.as_ptr() as *mut c_void,
						memory::PAGE_SIZE);
				}
				Some(cow_buffer)
			} else {
				None
			}
		};

		// TODO Separate allocated and non-allocated pointer
		let phys_ptr = {
			if let Some(ptr) = self.get_physical_page(offset, vmem) {
				if cow {
					buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?
				} else {
					ptr
				}
			} else {
				buddy::alloc(0, buddy::FLAG_ZONE_TYPE_USER)?
			}
		};

		let flags = self.get_vmem_flags(true);
		if let Err(errno) = vmem.map(phys_ptr, virt_ptr, flags) {
			buddy::free(phys_ptr, 0); // TODO Fix: UB if freeing the pointer that's already mapped
			return Err(errno);
		}
		vmem.flush();

		stack_switch(tmp_stack.as_mut_ptr() as _,
			| data: &(&mut Box::<dyn VMem>,
				*mut c_void,
				Option::<Box::<[u8; memory::PAGE_SIZE]>>) | {
				let vmem = &*ManuallyDrop::new(unsafe { // Call to unsafe function
					Box::from_raw(data.0.as_mut_ptr())
				});
				let virt_ptr = data.1;
				let buffer = &data.2;

				vmem_switch(vmem, move || {
					if let Some(buffer) = &buffer {
						unsafe { // Call to unsafe functions
							ptr::copy_nonoverlapping(buffer.as_ptr() as *const c_void,
								virt_ptr as *mut c_void,
								memory::PAGE_SIZE);
						}
					} else {
						unsafe { // Call to unsafe function
							util::bzero(virt_ptr as _, memory::PAGE_SIZE);
						}
					}
				});
			} as _, (vmem, virt_ptr, cow_buffer))?;

		if cow {
			unsafe { // Call to unsafe function
				self.shared_list.unlink_floating();
			}
		}

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
