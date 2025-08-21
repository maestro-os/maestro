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

//! The virtual memory makes the kernel able to isolate processes, which is
//! essential for modern systems.

use crate::{
	acpi::{RSDP_SCAN_BEGIN, RSDP_SCAN_END},
	arch::{
		x86,
		x86::{
			paging::{FLAG_CACHE_DISABLE, FLAG_GLOBAL, FLAG_USER, FLAG_WRITE, FLAG_WRITE_THROUGH},
			smp,
		},
	},
	elf,
	elf::SHF_WRITE,
	memory::{KERNEL_BEGIN, PhysAddr, VirtAddr, buddy, memmap::mmap_iter},
	multiboot::{MEMORY_ACPI_RECLAIMABLE, MEMORY_AVAILABLE, MEMORY_RESERVED},
	process::scheduler::{CPU, defer},
	sync::{mutex::Mutex, once::OnceInit},
	tty::vga,
};
use core::{ptr::NonNull, sync::atomic::Ordering::Release};
use core::sync::atomic::Ordering::{Acquire};
use utils::limits::PAGE_SIZE;

/// A virtual memory context.
///
/// This structure implements operations to modify virtual memory in an architecture-independent
/// way.
///
/// Internally, the structure retries allocations on failure, to avoid returning allocation errors.
/// This greatly reduces the complexity of the kernel.
pub struct VMem {
	/// The root paging object.
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	table: NonNull<x86::paging::Table>,
}

impl VMem {
	/// Creates a new virtual memory context.
	///
	/// # Safety
	///
	/// Modifying kernel mappings might result in an undefined behaviour. It is the caller's
	/// responsibility to ensure code and data (including stacks) remain accessible.
	pub unsafe fn new() -> Self {
		Self {
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			table: x86::paging::alloc(),
		}
	}

	/// Returns an immutable reference to the **architecture-dependent** inner representation.
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	pub fn inner(&self) -> &x86::paging::Table {
		unsafe { self.table.as_ref() }
	}

	/// Returns a mutable reference to the architecture-dependent inner representation.
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	pub fn inner_mut(&mut self) -> &mut x86::paging::Table {
		unsafe { self.table.as_mut() }
	}

	/// Translates the given virtual address `addr` to the corresponding physical
	/// address.
	///
	/// If the address is not mapped, the function returns `None`.
	pub fn translate(&self, addr: VirtAddr) -> Option<PhysAddr> {
		#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
		x86::paging::translate(self.inner(), addr)
	}

	/// Maps a single page of virtual memory at `virtaddr` to a single page of physical memory at
	/// `physaddr`.
	///
	/// `flags` is the set of flags to use for the mapping, which are architecture-dependent.
	#[inline]
	pub fn map(&mut self, physaddr: PhysAddr, virtaddr: VirtAddr, flags: usize) {
		#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
		unsafe {
			x86::paging::map(self.inner_mut(), physaddr, virtaddr, flags);
		}
		invalidate_page(virtaddr);
	}

	/// Like [`Self::map`] but on a range of several pages.
	///
	/// On overflow, the physical and virtual addresses wrap around the userspace.
	pub fn map_range(
		&mut self,
		physaddr: PhysAddr,
		virtaddr: VirtAddr,
		pages: usize,
		flags: usize,
	) {
		for i in 0..pages {
			let physaddr = physaddr + i * PAGE_SIZE;
			let virtaddr = virtaddr + i * PAGE_SIZE;
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			unsafe {
				x86::paging::map(self.inner_mut(), physaddr, virtaddr, flags);
			}
		}
		invalidate_range(virtaddr, pages);
	}

	/// Unmaps a single page of virtual memory at `virtaddr`.
	#[inline]
	pub fn unmap(&mut self, virtaddr: VirtAddr) {
		#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
		unsafe {
			x86::paging::unmap(self.inner_mut(), virtaddr);
		}
		invalidate_page(virtaddr);
	}

	/// Like [`Self::unmap`] but on a range of several pages.
	///
	/// On overflow, the physical and virtual addresses wrap around the userspace.
	pub fn unmap_range(&mut self, virtaddr: VirtAddr, pages: usize) {
		for i in 0..pages {
			let virtaddr = virtaddr + i * PAGE_SIZE;
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			unsafe {
				x86::paging::unmap(self.inner_mut(), virtaddr);
			}
		}
		invalidate_range(virtaddr, pages);
	}

	/// Polls the dirty flags on the range of `pages` pages starting at `addr`, clearing them
	/// atomically, and setting them to the associated [`buddy::Page`] structure.
	pub fn poll_dirty(&self, addr: VirtAddr, pages: usize) {
		for n in 0..pages {
			// TODO polling pages one by one is inefficient
			let addr = addr + n * PAGE_SIZE;
			let Some((physaddr, true)) = x86::paging::poll_dirty(self.inner(), addr) else {
				continue;
			};
			let page = buddy::get_page(physaddr);
			page.dirty.store(true, Release);
		}
	}

	/// Binds the virtual memory context to the current CPU.
	pub fn bind(&self) {
		let phys_addr = VirtAddr::from(self.table.as_ptr())
			.kernel_to_physical()
			.unwrap();
		unsafe {
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			x86::paging::bind(phys_addr);
		}
	}

	/// Tells whether the context is bound to the current CPU.
	pub fn is_bound(&self) -> bool {
		#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
		x86::paging::is_bound(self.table)
	}
}

impl Drop for VMem {
	fn drop(&mut self) {
		if self.is_bound() {
			panic!("Dropping virtual memory context while in use!");
		}
		#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
		unsafe {
			x86::paging::free(self.table);
		}
	}
}

/// Invalidate the page from cache at the given address on the current CPU.
#[inline]
pub fn invalidate_page(addr: VirtAddr) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::paging::invlpg(addr);
}

/// Invalidate the range of `count` pages starting at `addr` on the current CPU.
pub fn invalidate_range(addr: VirtAddr, count: usize) {
	for i in 0..count {
		invalidate_page(addr + i * PAGE_SIZE);
	}
}

/// Flush the Translation Lookaside Buffer (TLB) on the current CPU.
///
/// This function should be called after applying modifications to the context for them to be
/// taken into account.
///
/// This is an expensive operation for the CPU cache and should be used as few as possible.
#[inline]
pub fn flush() {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::paging::flush();
}

// TODO shootdown only cores that are sharing the same memory space, unless we are invalidation kernel mappings (in which case we shall shootdown everyone)

/// Invalidate the page at `addr` on all CPUs.
pub fn shootdown_page(addr: VirtAddr) {
	CPU.iter()
		.filter(|cpu| cpu.online.load(Acquire))
		.for_each(|cpu| defer::synchronous(cpu.apic_id, move || invalidate_page(addr)));
}

/// Invalidate the range of `count` pages starting at `addr` on all CPUs.
pub fn shootdown_range(addr: VirtAddr, count: usize) {
	CPU.iter()
		.filter(|cpu| cpu.online.load(Acquire))
		.for_each(|cpu| defer::synchronous(cpu.apic_id, move || invalidate_range(addr, count)));
}

/// Executes the closure while allowing the kernel to write on read-only pages.
///
/// # Safety
///
/// This function disables memory protection on the kernel side, which makes
/// read-only data writable.
///
/// Writing on some read-only regions (code for example) is dangerous.
#[inline]
pub unsafe fn write_ro<F: FnOnce() -> T, T>(f: F) -> T {
	let prev = x86::set_write_protected(false);
	let res = f();
	x86::set_write_protected(prev);
	res
}

/// Executes the closure while allowing the kernel to access user data by disabling SMAP.
///
/// # Safety
///
/// SMAP provides a security against potentially malicious data accesses. As such, it should be
/// disabled only when strictly necessary.
///
/// Enabling SMAP removes access to memory addresses that were previously accessible. It is the
/// caller's responsibility to ensure no invalid memory accesses are done afterward.
#[inline]
pub unsafe fn smap_disable<F: FnOnce() -> T, T>(f: F) -> T {
	x86::set_smap_enabled(false);
	let res = f();
	x86::set_smap_enabled(true);
	res
}

/// The kernel's virtual memory context.
pub static KERNEL_VMEM: OnceInit<Mutex<VMem>> = unsafe { OnceInit::new() };

/// Initializes virtual memory management.
pub(crate) fn init() {
	// Kernel context init
	let mut kernel_vmem = unsafe { VMem::new() };
	// Map kernel
	for entry in mmap_iter() {
		if !matches!(
			entry.type_,
			MEMORY_AVAILABLE | MEMORY_RESERVED | MEMORY_ACPI_RECLAIMABLE
		) {
			continue;
		}
		// If the address overflows an usize, the mapping cannot be reached
		let addr: Option<usize> = (KERNEL_BEGIN.0 as u64)
			.checked_add(entry.addr)
			.and_then(|end| end.try_into().ok());
		let Some(addr) = addr else {
			continue;
		};
		// Compute the maximum size of the mapping fitting an usize
		let end = (addr as u64).saturating_add(entry.len);
		let len = (end - addr as u64).min(usize::MAX as u64) as usize;
		kernel_vmem.map_range(
			PhysAddr(entry.addr as _),
			VirtAddr(addr),
			len.div_ceil(PAGE_SIZE),
			FLAG_WRITE | FLAG_GLOBAL,
		);
	}
	// Make the kernel's code read-only
	let iter = elf::kernel::sections().filter(|s| s.sh_addralign as usize == PAGE_SIZE);
	for section in iter {
		let write = section.sh_flags as u32 & SHF_WRITE != 0;
		let user = elf::kernel::get_section_name(&section) == Some(b".user");
		let mut flags = FLAG_GLOBAL;
		if write {
			flags |= FLAG_WRITE;
		}
		if user {
			flags |= FLAG_USER;
		}
		// Map
		let virt_addr = if section.sh_addr as usize >= KERNEL_BEGIN.0 {
			VirtAddr(section.sh_addr as _)
		} else {
			KERNEL_BEGIN + section.sh_addr as _
		};
		let Some(phys_addr) = virt_addr.kernel_to_physical() else {
			continue;
		};
		let pages = section.sh_size.div_ceil(PAGE_SIZE as _) as usize;
		kernel_vmem.map_range(phys_addr, virt_addr, pages, flags);
	}
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	{
		// Map SMP trampoline
		kernel_vmem.map(
			smp::TRAMPOLINE_PHYS_ADDR,
			VirtAddr(smp::TRAMPOLINE_PHYS_ADDR.0),
			0,
		);
		kernel_vmem.map(
			smp::TRAMPOLINE_PHYS_ADDR,
			smp::TRAMPOLINE_PHYS_ADDR.kernel_to_virtual().unwrap(),
			0,
		);
		// Map VGA buffer
		kernel_vmem.map_range(
			vga::BUFFER_PHYS as _,
			vga::get_buffer_virt().into(),
			1,
			FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH | FLAG_WRITE | FLAG_GLOBAL,
		);
		// Ensure ACPI RSDP is mapped
		kernel_vmem.map_range(
			PhysAddr(RSDP_SCAN_BEGIN),
			KERNEL_BEGIN + RSDP_SCAN_BEGIN,
			(RSDP_SCAN_END + 1 - RSDP_SCAN_BEGIN) / PAGE_SIZE,
			FLAG_GLOBAL,
		);
	}
	kernel_vmem.bind();
	unsafe {
		OnceInit::init(&KERNEL_VMEM, Mutex::new(kernel_vmem));
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn vmem_basic0() {
		let vmem = unsafe { VMem::new() };
		for i in (0..0xc0000000).step_by(PAGE_SIZE) {
			assert_eq!(vmem.translate(VirtAddr(i)), None);
		}
	}

	#[test_case]
	fn vmem_map0() {
		let mut vmem = unsafe { VMem::new() };
		vmem.map(PhysAddr(0x100000), VirtAddr(0x100000), 0);
		for i in (0..0xc0000000).step_by(PAGE_SIZE) {
			let res = vmem.translate(VirtAddr(i));
			if (0x100000..0x101000).contains(&i) {
				assert_eq!(res, Some(PhysAddr(i)));
			} else {
				assert_eq!(res, None);
			}
		}
	}

	#[test_case]
	fn vmem_map1() {
		let mut vmem = unsafe { VMem::new() };
		vmem.map(PhysAddr(0x100000), VirtAddr(0x100000), 0);
		vmem.map(PhysAddr(0x200000), VirtAddr(0x100000), 0);
		for i in (0..0xc0000000).step_by(PAGE_SIZE) {
			let res = vmem.translate(VirtAddr(i));
			if (0x100000..0x101000).contains(&i) {
				assert_eq!(res, Some(PhysAddr(0x100000 + i)));
			} else {
				assert_eq!(res, None);
			}
		}
	}

	#[test_case]
	fn vmem_unmap0() {
		let mut vmem = unsafe { VMem::new() };
		vmem.map(PhysAddr(0x100000), VirtAddr(0x100000), 0);
		vmem.unmap(VirtAddr(0x100000));
		for i in (0..0xc0000000).step_by(PAGE_SIZE) {
			assert_eq!(vmem.translate(VirtAddr(i)), None);
		}
	}
}
