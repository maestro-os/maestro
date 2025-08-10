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

use super::{PhysAddr, VirtAddr, buddy};
use crate::{
	arch::x86::paging::{FLAG_CACHE_DISABLE, FLAG_GLOBAL, FLAG_WRITE, FLAG_WRITE_THROUGH},
	memory::{buddy::ZONE_MMIO, vmem::KERNEL_VMEM},
};
use core::{num::NonZeroUsize, ptr::NonNull};
use utils::{errno::AllocResult, limits::PAGE_SIZE};
// TODO allow usage of virtual memory that isn't linked to any physical pages

/// MMIO registers
///
/// If the requested physical memory is not reachable, memory is allocated to remap it. This memory
/// is freed when the structure is dropped.
#[derive(Debug)]
pub struct Mmio {
	/// The physical address.
	phys_addr: PhysAddr,
	/// The virtual address.
	virt_addr: *mut u8,
	/// The number of mapped pages.
	pages: NonZeroUsize,
}

impl Mmio {
	/// Maps `phys_addr` to be usable for MMIO.
	///
	/// Arguments:
	/// - `phys_addr` is the address in physical memory to the chunk to be mapped
	/// - `pages` is the number of pages to be mapped
	/// - `prefetchable` tells whether memory can be prefeteched
	///
	/// If `phys_addr` is outside reachable physical memory, the function allocates memory to remap
	/// it.
	///
	/// If not enough physical or virtual memory is available, the function returns an error.
	pub fn new(phys_addr: PhysAddr, pages: NonZeroUsize, prefetchable: bool) -> AllocResult<Self> {
		// If the address is out of the reachable range, allocate memory for it
		let last_page = phys_addr + (pages.get() - 1) * PAGE_SIZE;
		let virt_addr = if last_page.kernel_to_virtual().is_none() {
			let order = buddy::get_order(pages);
			buddy::alloc(order, ZONE_MMIO)?.kernel_to_virtual().unwrap()
		} else {
			phys_addr.kernel_to_virtual().unwrap()
		};
		// Remap
		let mut flags = FLAG_WRITE | FLAG_WRITE_THROUGH | FLAG_GLOBAL;
		if !prefetchable {
			flags |= FLAG_CACHE_DISABLE;
		}
		KERNEL_VMEM
			.lock()
			.map_range(phys_addr, virt_addr, pages.get(), flags);
		// Beginning offset in the page
		let page_off = phys_addr.0 & 0xfff;
		let virt_addr = unsafe { virt_addr.as_ptr::<u8>().add(page_off) };
		Ok(Self {
			phys_addr,
			virt_addr,
			pages,
		})
	}

	/// Returns a pointer to the beginning of the MMIO chunk.
	#[inline]
	pub fn as_ptr<T>(&self) -> *mut T {
		self.virt_addr.cast()
	}
}

impl Drop for Mmio {
	fn drop(&mut self) {
		// Restore mapping
		KERNEL_VMEM.lock().map_range(
			self.phys_addr,
			VirtAddr::from(self.virt_addr),
			self.pages.get(),
			FLAG_WRITE | FLAG_GLOBAL,
		);
		// Free allocated virtual pages
		let order = buddy::get_order(self.pages);
		unsafe {
			buddy::free_kernel(self.virt_addr.as_ptr(), order);
		}
	}
}
