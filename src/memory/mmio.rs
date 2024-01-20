//! MMIO (Memory-Mapped I/O) allows to access a device's registers by mapping them on the main
//! memory.

use super::buddy;
use super::vmem;
use crate::errno::AllocResult;
use crate::process::oom;
use core::ffi::c_void;

/// Default flags for kernelspace in virtual memory.
const DEFAULT_FLAGS: u32 = vmem::x86::FLAG_WRITE;

/// MMIO flags in virtual memory.
const MMIO_FLAGS: u32 =
	vmem::x86::FLAG_WRITE_THROUGH | vmem::x86::FLAG_WRITE | vmem::x86::FLAG_GLOBAL;

// TODO allow usage of virtual memory that isn't linked to any physical pages

/// Structure representing the mapping of a chunk of memory for MMIO.
#[derive(Debug)]
pub struct MMIO {
	/// The physical address.
	phys_addr: *mut c_void,
	/// The virtual address.
	virt_addr: *mut c_void,

	/// The number of mapped pages.
	pages: usize,
}

impl MMIO {
	/// Creates and maps a new MMIO chunk.
	///
	/// Arguments:
	/// - `phys_addr` is the address in physical memory to the chunk to be mapped.
	/// - `pages` is the number of pages to be mapped.
	/// - `prefetchable` tells whether memory can be prefeteched.
	///
	/// The virtual address is allocated by this function.
	///
	/// If not enough physical or virtual memory is available, the function returns an error.
	#[allow(clippy::not_unsafe_ptr_arg_deref)]
	pub fn new(phys_addr: *mut c_void, pages: usize, prefetchable: bool) -> AllocResult<Self> {
		let order = buddy::get_order(pages);
		let virt_addr = buddy::alloc_kernel(order)?;

		let mut flags = MMIO_FLAGS;
		if !prefetchable {
			flags |= vmem::x86::FLAG_CACHE_DISABLE;
		}

		let mut vmem = crate::get_vmem().lock();
		unsafe {
			vmem.as_mut()
				.unwrap()
				.map_range(phys_addr, virt_addr.as_ptr(), pages, flags)?;
		}

		Ok(Self {
			phys_addr,
			virt_addr: virt_addr.as_ptr(),

			pages,
		})
	}

	/// Returns an immutable pointer to the virtual address of the chunk.
	pub fn as_ptr(&self) -> *const c_void {
		self.virt_addr
	}

	/// Returns an immutable pointer to the virtual address of the chunk.
	pub fn as_mut_ptr(&mut self) -> *mut c_void {
		self.virt_addr
	}

	/// Unmaps the MMIO chunk.
	///
	/// The previously allocated chunk is freed by this function.
	pub fn unmap(&self) -> AllocResult<()> {
		let mut vmem = crate::get_vmem().lock();
		unsafe {
			vmem.as_mut().unwrap().map_range(
				self.phys_addr,
				super::kern_to_virt(self.phys_addr),
				self.pages,
				DEFAULT_FLAGS,
			)?;
		}

		let order = buddy::get_order(self.pages);
		buddy::free_kernel(self.phys_addr, order);

		Ok(())
	}
}

impl Drop for MMIO {
	fn drop(&mut self) {
		oom::wrap(|| self.unmap());
	}
}
