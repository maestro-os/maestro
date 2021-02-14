/// This module implements the MemSpace structure which is responsible for handling the memory
/// mapping of execution contexts.
/// TODO doc

pub const MAPPING_FLAG_READ: u8 = 0b001;
pub const MAPPING_FLAG_WRITE: u8 = 0b010;
pub const MAPPING_FLAG_EXEC: u8 = 0b100;

use core::ffi::c_void;
use crate::memory::vmem::MutVMem;

/// Structure representing the virtual memory of a context.
pub struct MemSpace {
	// TODO Store memory mappings and gaps

	/// The architecture-dependent paging object.
	paging_context: MutVMem, // TODO Use a wrapper to Drop automatically
}

impl MemSpace {
	/// Creates a new virtual memory object.
	pub fn new() -> Self {
		Self {
			paging_context: 0 as _, // TODO
		}
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
	pub fn map(_ptr: Option::<*const c_void>, _size: usize, _flags: u8) -> Result::<c_void, ()> {
		// TODO
		Err(())
	}

	/// Unmaps the given region of memory.
	/// `ptr` represents the address of the beginning of the region on the virtual memory.
	/// `size` represents the size of the region in number of memory pages.
	/// The function frees the physical memory the region points to unless shared by one or several
	/// other memory mappings.
	/// After this function returns, the access to the region of memory shall be revoked and
	/// further attempts to access it shall result in a page fault.
	pub fn unmap(_ptr: *const c_void, _size: usize) {
		// TODO
	}

	/// Function called whenever the CPU triggered a page fault for the context. This function
	/// determines whether the process should continue or not. If continuing, the function must
	/// resolve the issue before returning.
	/// A typical situation where is function is usefull is for Copy-On-Write allocations.
	///
	/// `virt_addr` is the virtual address of the wrong memory access that caused the fault.
	/// If the process should continue, the function returns `true`, else `false`.
	pub fn handle_page_fault(_virt_addr: *const c_void) -> bool {
		// TODO
		false
	}
}
