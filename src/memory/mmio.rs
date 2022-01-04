//! This module implements utility allowing to control Memory Mapped I/O (MMIO).

use core::ffi::c_void;
use core::slice;
use crate::errno::Errno;
use crate::memory::vmem;
use crate::memory;

/// Structure representing a MMIO region.
pub struct MMIO {
	/// The physical address to the beginning of the MMIO.
	phys_begin: *mut c_void,
	/// The virtual address to the beginning of the MMIO.
	virt_begin: *mut c_void,

	/// The size of the MMIO in pages.
	pages: usize,
}

impl MMIO {
	/// Maps the MMIO's memory.
	fn map(&self) -> Result<(), Errno> {
		let mut paging_lock = crate::get_vmem().lock();
		let paging = paging_lock.get_mut().as_mut().unwrap();

		// Mapping the memory
		let flags = vmem::x86::FLAG_GLOBAL | vmem::x86::FLAG_CACHE_DISABLE
			| vmem::x86::FLAG_WRITE_THROUGH | vmem::x86::FLAG_WRITE;
		paging.map_range(self.phys_begin, self.virt_begin, self.pages, flags)?;

		Ok(())
	}

	/// Creates a new instance.
	pub fn new(phys_begin: *mut c_void, pages: usize) -> Result<Self, Errno> {
		// Allocating virtual memory
		let virt_begin = phys_begin; // TODO Allocate

		let s = Self {
			phys_begin,
			virt_begin,

			pages,
		};
		s.map()?;
		Ok(s)
	}

	/// Returns the physical address of the beginning of the MMIO.
	#[inline(always)]
	pub fn get_phys_begin(&self) -> *mut c_void {
		self.phys_begin
	}

	/// Returns the virtual address of the beginning of the MMIO.
	#[inline(always)]
	pub fn get_virt_begin(&self) -> *mut c_void {
		self.virt_begin
	}

	/// Returns the number of pages spanned by the MMIO.
	#[inline(always)]
	pub fn get_pages(&self) -> usize {
		self.pages
	}

	/// Returns a slice to the MMIO.
	pub fn get_slice(&self) -> &[u8] {
		let len = self.pages * memory::PAGE_SIZE;

		unsafe { // Safe because the memory is mapped
			slice::from_raw_parts(self.virt_begin as _, len)
		}
	}

	/// Returns a slice to the MMIO.
	pub fn get_slice_mut(&mut self) -> &mut [u8] {
		let len = self.pages * memory::PAGE_SIZE;

		unsafe { // Safe because the memory is mapped
			slice::from_raw_parts_mut(self.virt_begin as _, len)
		}
	}
}

impl Drop for MMIO {
	fn drop(&mut self) {
		// TODO Unmap?
	}
}
