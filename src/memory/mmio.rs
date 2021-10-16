//! This module implements utility allowing to control Memory Mapped I/O (MMIO).

use core::ffi::c_void;
use core::slice;
use crate::errno::Errno;
use crate::memory::vmem::VMem;
use crate::memory;

/// A view of a MMIO region, allowing to access it.
pub struct MMIOView<'a> {
	/// The reference to the MMIO.
	mmio: &'a mut MMIO,
}

impl<'a> MMIOView<'a> {
	/// Creates a new instance for the given MMIO.
	fn new(mmio: &'a mut MMIO) -> Self {
		Self {
			mmio,
		}
	}

	/// Returns a slice to the MMIO.
	pub fn get_slice(&self) -> &[u8] {
		let len = self.mmio.pages * memory::PAGE_SIZE;

		unsafe {
			slice::from_raw_parts(self.mmio.virt_begin as _, len)
		}
	}

	/// Returns a slice to the MMIO.
	pub fn get_slice_mut(&mut self) -> &mut [u8] {
		let len = self.mmio.pages * memory::PAGE_SIZE;

		unsafe {
			slice::from_raw_parts_mut(self.mmio.virt_begin as _, len)
		}
	}
}

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
	/// Creates a new instance.
	pub fn new(phys_begin: *mut c_void, pages: usize) -> Result<Self, Errno> {
		let virt_begin = phys_begin; // TODO Allocate

		Ok(Self {
			phys_begin,
			virt_begin,

			pages,
		})
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

	/// Maps the MMIO to the given paging context.
	pub fn get_view(&mut self, paging: &mut dyn VMem) -> MMIOView {
		// TODO

		paging.flush();
		MMIOView::new(self)
	}
}
