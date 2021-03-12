/// This module implements the MemSpace structure which is responsible for handling the memory
/// mapping of execution contexts.
/// TODO doc

use core::ffi::c_void;
use crate::memory::vmem::VMem;
use crate::memory::vmem;
use crate::memory;
use crate::util::boxed::Box;
use crate::util::container::binary_tree::BinaryTree;

/// Flag telling that a memory mapping can be read from.
pub const MAPPING_FLAG_READ: u8   = 0b000001;
/// Flag telling that a memory mapping can be written to.
pub const MAPPING_FLAG_WRITE: u8  = 0b000010;
/// Flag telling that a memory mapping can contain executable instructions.
pub const MAPPING_FLAG_EXEC: u8   = 0b000100;
/// Flag telling that a memory mapping is accessible from userspace.
pub const MAPPING_FLAG_USER: u8   = 0b001000;
/// Flag telling that a memory mapping must allocate its physical memory right away and not when
/// the process tries to write to it.
pub const MAPPING_FLAG_NOLAZY: u8 = 0b010000;
/// Flag telling that a memory mapping has its physical memory shared with one or more other
/// mappings.
pub const MAPPING_FLAG_SHARED: u8 = 0b100000;

/// A gap in the memory space that can use for new mappings.
pub struct MemGap {
	/// Pointer on the virtual memory to the beginning of the gap
	begin: *const c_void,
	/// The size of the gap in pages.
	size: usize,
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

impl MemMapping {
	/// Creates a new instance.
	/// `begin` is the pointer on the virtual memory to the beginning of the mapping.
	/// `size` is the size of the mapping in pages.
	/// `flags` the mapping's flags
	pub fn new(begin: *const c_void, size: usize, flags: u8) -> Self {
		Self {
			begin: begin,
			size: size,
			flags: flags,
		}
	}

	// TODO
}

/// Structure representing the virtual memory space of a context.
pub struct MemSpace {
	/// Binary tree storing the list of memory gaps, ready for new mappings. Sorted by size.
	gaps: BinaryTree::<MemGap>,
	/// Binary tree storing the list of memory mappings. Sorted by pointer to the beginning of the
	/// mapping on the virtual memory.
	mappings: BinaryTree::<MemMapping>,

	/// The virtual memory context handler.
	vmem: Box::<dyn VMem>,
}

impl MemSpace {
	/// Creates a new virtual memory object.
	pub fn new() -> Result::<Self, ()> {
		Ok(Self {
			gaps: BinaryTree::new(),
			mappings: BinaryTree::new(),

			vmem: vmem::new()?,
		})
	}

	/// Maps a region of memory.
	/// `ptr` represents the address of the beginning of the region on the virtual memory.
	/// If the address is None, the function shall find a gap in the memory space that is large
	/// enough to contain the mapping.
	/// `size` represents the size of the region in number of memory pages.
	/// `flags` represents the flags for the mapping.
	/// underlying physical memory is not allocated directly but only an attempt to write the
	/// memory is detected.
	/// The function returns a pointer to the newly mapped virtual memory.
	pub fn map(&mut self, ptr: Option::<*const c_void>, size: usize, flags: u8)
		-> Result::<*const c_void, ()> {
		if let Some(_ptr) = ptr {
			// TODO Insert mapping at exact location if possible
			Err(())
		} else {
			let gap = self.gaps.get(| val | {
				size.cmp(&val.size)
			});

			if let Some(gap) = gap {
				let mapping = MemMapping::new(gap.begin, size, flags);
				self.mappings.insert(mapping, | n0, n1 | {
					n0.begin.cmp(&n1.begin)
				})?;

				// TODO Create a gap with the new size and location and insert it
				// If the insertion fails, remove the new mapping form the tree and return Err
				// TODO Remove the old gap from the tree

				Err(()) // TODO Return Ok
			} else {
				Err(())
			}
		}
	}

	/// Same as `map`, except the function returns a pointer to the end of the memory region.
	pub fn map_stack(&mut self, ptr: Option::<*const c_void>, size: usize, flags: u8)
		-> Result::<*const c_void, ()> {
		let mapping_ptr = self.map(ptr, size, flags)?;
		Ok(unsafe { // Call to unsafe function
			mapping_ptr.add(size * memory::PAGE_SIZE) // `- 1`?
		})
	}

	/// Unmaps the given region of memory.
	/// `ptr` represents the address of the beginning of the region on the virtual memory.
	/// `size` represents the size of the region in number of memory pages.
	/// The function frees the physical memory the region points to unless shared by one or several
	/// other memory mappings.
	/// After this function returns, the access to the region of memory shall be revoked and
	/// further attempts to access it shall result in a page fault.
	pub fn unmap(&mut self, _ptr: *const c_void, _size: usize) {
		// TODO
	}

	/// Binds the CPU to this memory space.
	pub fn bind(&self) {
		self.vmem.bind();
	}

	/// Function called whenever the CPU triggered a page fault for the context. This function
	/// determines whether the process should continue or not. If continuing, the function must
	/// resolve the issue before returning.
	/// A typical situation where is function is usefull is for Copy-On-Write allocations.
	///
	/// `virt_addr` is the virtual address of the wrong memory access that caused the fault.
	/// If the process should continue, the function returns `true`, else `false`.
	pub fn handle_page_fault(&self, _virt_addr: *const c_void) -> bool {
		// TODO
		false
	}
}

impl Drop for MemSpace {
	fn drop(&mut self) {
		// TODO Free every allocations
	}
}
