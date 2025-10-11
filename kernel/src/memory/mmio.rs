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
	memory::{
		buddy::ZONE_MMIO,
		vmem::{KERNEL_VMEM, shootdown_range},
	},
	process::scheduler::cpu::iter_online,
};
use core::num::NonZeroUsize;
use utils::{errno::AllocResult, limits::PAGE_SIZE};

/// MMIO registers
///
/// If the requested physical memory is not reachable, memory is allocated to remap it. This memory
/// is freed when the structure is dropped.
#[derive(Debug)]
pub struct Mmio {
	/// Allocated physical address, if any. This physical memory is not used, we only need its
	/// associated virtual address
	phys_addr: Option<PhysAddr>,
	/// The virtual address
	virt_addr: *mut u8,
	/// The number of mapped pages
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
		let (allocated_phys_addr, virt_addr) = if last_page.kernel_to_virtual().is_none() {
			let order = buddy::get_order(pages);
			let allocated = buddy::alloc(order, ZONE_MMIO)?;
			(Some(allocated), allocated.kernel_to_virtual().unwrap())
		} else {
			(None, phys_addr.kernel_to_virtual().unwrap())
		};
		// Remap
		let mut flags = FLAG_WRITE | FLAG_WRITE_THROUGH | FLAG_GLOBAL;
		if !prefetchable {
			flags |= FLAG_CACHE_DISABLE;
		}
		KERNEL_VMEM.map_range(phys_addr, virt_addr, pages.get(), flags);
		shootdown_range(virt_addr, pages.get(), iter_online());
		// Add offset to virtual address
		let page_off = phys_addr.0 & 0xfff;
		let virt_addr = unsafe { virt_addr.as_ptr::<u8>().add(page_off) };
		Ok(Self {
			phys_addr: allocated_phys_addr,
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
		let virt_addr = VirtAddr::from(self.virt_addr);
		KERNEL_VMEM.map_range(
			virt_addr.kernel_to_physical().unwrap(),
			virt_addr,
			self.pages.get(),
			FLAG_WRITE | FLAG_GLOBAL,
		);
		shootdown_range(virt_addr, self.pages.get(), iter_online());
		// Free allocated physical memory, if any
		if let Some(phys_addr) = self.phys_addr {
			let order = buddy::get_order(self.pages);
			unsafe {
				buddy::free(phys_addr, order);
			}
		}
	}
}
