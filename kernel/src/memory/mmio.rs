/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! MMIO (Memory-Mapped I/O) allows to access a device's registers by mapping them on the main
//! memory.

use super::{buddy, oom, PhysAddr, VirtAddr};
use crate::{arch::x86, memory::vmem::KERNEL_VMEM};
use core::ptr::NonNull;
use utils::errno::AllocResult;

/// Default flags for kernelspace in virtual memory.
const DEFAULT_FLAGS: x86::paging::Entry = x86::paging::FLAG_WRITE;

/// MMIO flags in virtual memory.
const MMIO_FLAGS: x86::paging::Entry =
	x86::paging::FLAG_WRITE_THROUGH | x86::paging::FLAG_WRITE | x86::paging::FLAG_GLOBAL;

// TODO allow usage of virtual memory that isn't linked to any physical pages

/// The mapping of a chunk of memory for MMIO.
#[derive(Debug)]
pub struct MMIO {
	/// The physical address.
	phys_addr: PhysAddr,
	/// The virtual address.
	virt_addr: VirtAddr,

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
	pub fn new(phys_addr: PhysAddr, pages: usize, prefetchable: bool) -> AllocResult<Self> {
		let order = buddy::get_order(pages);
		let virt_addr = buddy::alloc_kernel(order)?.into();

		let mut flags = MMIO_FLAGS;
		if !prefetchable {
			flags |= x86::paging::FLAG_CACHE_DISABLE;
		}

		let mut vmem = KERNEL_VMEM.lock();
		let mut transaction = vmem.transaction();
		transaction.map_range(phys_addr, virt_addr, pages, flags)?;
		transaction.commit();

		Ok(Self {
			phys_addr,
			virt_addr,

			pages,
		})
	}

	/// Returns the pointer to the beginning of the MMIO chunk.
	pub fn as_ptr(&self) -> NonNull<u8> {
		NonNull::new(self.virt_addr.as_ptr()).unwrap()
	}

	/// Unmaps the MMIO chunk.
	///
	/// The previously allocated chunk is freed by this function.
	pub fn unmap(&self) -> AllocResult<()> {
		let mut vmem = KERNEL_VMEM.lock();
		let mut transaction = vmem.transaction();
		transaction.map_range(
			self.phys_addr,
			self.phys_addr.kernel_to_virtual().unwrap(),
			self.pages,
			DEFAULT_FLAGS,
		)?;
		transaction.commit();
		// Free allocated virtual pages
		let order = buddy::get_order(self.pages);
		unsafe {
			buddy::free_kernel(self.virt_addr.as_ptr(), order);
		}
		Ok(())
	}
}

impl Drop for MMIO {
	fn drop(&mut self) {
		oom::wrap(|| self.unmap());
	}
}
