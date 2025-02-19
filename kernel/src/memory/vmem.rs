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
	arch::{
		x86,
		x86::{
			idt,
			paging::{FLAG_CACHE_DISABLE, FLAG_GLOBAL, FLAG_USER, FLAG_WRITE, FLAG_WRITE_THROUGH},
		},
	},
	elf, memory,
	memory::{memmap::PHYS_MAP, PhysAddr, VirtAddr, KERNELSPACE_SIZE},
	register_get,
	sync::{mutex::Mutex, once::OnceInit},
	tty::vga,
};
use core::{alloc::AllocError, cmp::min, intrinsics::unlikely, mem, ptr::NonNull};
use utils::{collections::vec::Vec, errno::AllocResult, limits::PAGE_SIZE, vec};

/// Tells whether the given range of memory overlaps with the kernelspace.
///
/// Arguments:
/// - `virtaddr` is the start of the range.
/// - `pages` is the size of the range in pages.
fn is_kernelspace(virtaddr: VirtAddr, pages: usize) -> bool {
	let Some(end) = virtaddr.0.checked_add(pages * PAGE_SIZE) else {
		return true;
	};
	end > memory::KERNEL_BEGIN.0
}

/// A virtual memory context.
///
/// This structure implements operations to modify virtual memory in an architecture-independent
/// way.
///
/// `KERNEL` specifies whether mapping in kernelspace is allowed. If not allowed, trying to do it
/// results in an error.
pub struct VMem<const KERNEL: bool = false> {
	/// The root paging object.
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	table: NonNull<x86::paging::Table>,
}

impl VMem<false> {
	/// Creates a new virtual memory context.
	pub fn new() -> AllocResult<Self> {
		Ok(Self {
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			table: x86::paging::alloc()?,
		})
	}
}

impl VMem<true> {
	/// Creates a new virtual memory context which is allowed to modify kernelspace.
	///
	/// # Safety
	///
	/// The caller must ensure that modifying kernelspace keeps the code and stack accessible and
	/// valid. Failure to do so results in an undefined behaviour.
	pub unsafe fn new_kernel() -> AllocResult<Self> {
		Ok(Self {
			#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
			table: x86::paging::alloc()?,
		})
	}
}

impl<const KERNEL: bool> VMem<KERNEL> {
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

	/// Begins a transaction.
	pub fn transaction(&mut self) -> VMemTransaction<'_, KERNEL> {
		VMemTransaction {
			vmem: self,
			rollback: vec![],
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

impl<const KERNEL: bool> Drop for VMem<KERNEL> {
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

/// Handle allowing to roll back operations on a virtual memory context.
///
/// Dropping the transaction without committing rollbacks all modifications.
#[must_use = "A vmem transaction has to be committed or explicitly ignored"]
pub struct VMemTransaction<'v, const KERNEL: bool> {
	/// The virtual memory context on which the transaction applies.
	pub vmem: &'v mut VMem<KERNEL>,
	/// The vector of handles to roll back the whole transaction.
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	rollback: Vec<x86::paging::Rollback>,
}

impl<const KERNEL: bool> VMemTransaction<'_, KERNEL> {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	fn map_impl(
		&mut self,
		physaddr: PhysAddr,
		virtaddr: VirtAddr,
		flags: x86::paging::Entry,
	) -> AllocResult<x86::paging::Rollback> {
		let res = unsafe { x86::paging::map(self.vmem.inner_mut(), physaddr, virtaddr, flags) };
		invalidate_page_current(virtaddr);
		res
	}

	/// Maps a single page of virtual memory at `virtaddr` to a single page of physical memory at
	/// `physaddr`.
	///
	/// `flags` is the set of flags to use for the mapping, which are architecture-dependent.
	///
	/// The modifications may not be flushed to the cache. It is the caller's responsibility to
	/// ensure they are.
	#[inline]
	pub fn map(
		&mut self,
		physaddr: PhysAddr,
		virtaddr: VirtAddr,
		flags: x86::paging::Entry,
	) -> AllocResult<()> {
		// If kernelspace modification is disabled, error if mapping onto kernelspace
		if unlikely(!KERNEL && is_kernelspace(virtaddr, 1)) {
			return Err(AllocError);
		}
		let r = self.map_impl(physaddr, virtaddr, flags)?;
		self.rollback.push(r)
	}

	/// Like [`Self::map`] but on a range of several pages.
	///
	/// On overflow, the physical and virtual addresses wrap around the userspace.
	pub fn map_range(
		&mut self,
		physaddr: PhysAddr,
		virtaddr: VirtAddr,
		pages: usize,
		flags: x86::paging::Entry,
	) -> AllocResult<()> {
		if unlikely(pages == 0) {
			// No op
			return Ok(());
		}
		if pages == 1 {
			return self.map(physaddr, virtaddr, flags);
		}
		// If kernelspace modification is disabled, error if mapping onto kernelspace
		if unlikely(!KERNEL && is_kernelspace(virtaddr, pages)) {
			return Err(AllocError);
		}
		// Map each page
		self.rollback.reserve(pages)?;
		for i in 0..pages {
			let physaddr = physaddr + i * PAGE_SIZE;
			let virtaddr = virtaddr + i * PAGE_SIZE;
			let r = self.map_impl(physaddr, virtaddr, flags)?;
			self.rollback.push(r)?;
		}
		Ok(())
	}

	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	fn unmap_impl(&mut self, virtaddr: VirtAddr) -> x86::paging::Rollback {
		let res = unsafe { x86::paging::unmap(self.vmem.inner_mut(), virtaddr) };
		invalidate_page_current(virtaddr);
		res
	}

	/// Unmaps a single page of virtual memory at `virtaddr`.
	///
	/// The modifications may not be flushed to the cache. It is the caller's responsibility to
	/// ensure they are.
	#[inline]
	pub fn unmap(&mut self, virtaddr: VirtAddr) -> AllocResult<()> {
		// If kernelspace modification is disabled, error if unmapping onto kernelspace
		if unlikely(!KERNEL && is_kernelspace(virtaddr, 1)) {
			return Err(AllocError);
		}
		let r = self.unmap_impl(virtaddr);
		self.rollback.push(r)
	}

	/// Like [`Self::unmap`] but on a range of several pages.
	///
	/// On overflow, the physical and virtual addresses wrap around the userspace.
	pub fn unmap_range(&mut self, virtaddr: VirtAddr, pages: usize) -> AllocResult<()> {
		if unlikely(pages == 0) {
			// No op
			return Ok(());
		}
		if pages == 1 {
			return self.unmap(virtaddr);
		}
		// If kernelspace modification is disabled, error if unmapping onto kernelspace
		if unlikely(!KERNEL && is_kernelspace(virtaddr, pages)) {
			return Err(AllocError);
		}
		// Map each page
		self.rollback.reserve(pages)?;
		for i in 0..pages {
			let virtaddr = virtaddr + i * PAGE_SIZE;
			let r = self.unmap_impl(virtaddr);
			self.rollback.push(r)?;
		}
		Ok(())
	}

	/// Validates the transaction.
	pub fn commit(&mut self) {
		self.rollback.clear();
	}
}

impl<const KERNEL: bool> Drop for VMemTransaction<'_, KERNEL> {
	fn drop(&mut self) {
		let rollback = mem::take(&mut self.rollback);
		// Rollback in reverse order
		rollback
			.into_iter()
			.rev()
			.for_each(x86::paging::Rollback::rollback);
	}
}

/// Invalidate the page from cache at the given address on the current CPU.
#[inline]
pub fn invalidate_page_current(addr: VirtAddr) {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::paging::invlpg(addr);
}

/// Flush the Translation Lookaside Buffer (TLB) on the current CPU.
///
/// This function should be called after applying modifications to the context for them to be
/// taken into account.
///
/// This is an expensive operation for the CPU cache and should be used as few as possible.
pub fn flush_current() {
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::paging::flush_current();
}

/// Executes the closure while allowing the kernel to write on read-only pages.
///
/// # Safety
///
/// This function disables memory protection on the kernel side, which makes
/// read-only data writable.
///
/// Writing on read-only regions of memory has an undefined behavior.
#[inline]
pub unsafe fn write_ro<F: FnOnce() -> T, T>(f: F) -> T {
	x86::set_write_protected(false);
	let res = f();
	x86::set_write_protected(true);
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

/// Executes the given closure `f` while being bound to the given virtual memory
/// context `vmem`.
///
/// After execution, the function restores the previous context.
///
/// The function disables interruptions while executing the closure. This is due
/// to the fact that if interruptions were enabled, the scheduler would be able
/// to change the running process, and thus when resuming execution, the virtual
/// memory context would be changed to the process's context, making the
/// behaviour undefined.
///
/// # Safety
///
/// The caller must ensure that the stack is accessible in both the current and given virtual
/// memory contexts.
pub unsafe fn switch<F: FnOnce() -> T, T>(vmem: &VMem, f: F) -> T {
	idt::wrap_disable_interrupts(|| {
		if vmem.is_bound() {
			f()
		} else {
			// Get current vmem
			let page_dir = PhysAddr(register_get!("cr3"));
			// Bind temporary vmem
			vmem.bind();
			let result = f();
			// Restore previous vmem
			x86::paging::bind(page_dir);
			result
		}
	})
}

/// The kernel's virtual memory context.
pub static KERNEL_VMEM: OnceInit<Mutex<VMem<true>>> = unsafe { OnceInit::new() };

/// Initializes virtual memory management.
pub(crate) fn init() -> AllocResult<()> {
	// Architecture-specific init
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	x86::paging::prepare();
	// Kernel context init
	let mut kernel_vmem = unsafe { VMem::new_kernel()? };
	let mut transaction = kernel_vmem.transaction();
	// TODO If Meltdown mitigation is enabled, only allow read access to a stub of
	// the kernel for interrupts
	// Map kernel
	let kernelspace_size = min(PHYS_MAP.memory_size, KERNELSPACE_SIZE / PAGE_SIZE);
	transaction.map_range(
		PhysAddr::default(),
		memory::KERNEL_BEGIN,
		kernelspace_size,
		FLAG_WRITE | FLAG_GLOBAL,
	)?;
	// Make the kernel's code read-only
	let iter = elf::kernel::sections().filter(|s| s.sh_addralign as usize == PAGE_SIZE);
	for section in iter {
		let write = section.sh_flags as u32 & elf::SHF_WRITE != 0;
		let user = elf::kernel::get_section_name(section) == Some(b".user");
		let mut flags = FLAG_GLOBAL;
		if write {
			flags |= FLAG_WRITE;
		}
		if user {
			flags |= FLAG_USER;
		}
		// Map
		let virt_addr = VirtAddr(section.sh_addr as _);
		let Some(phys_addr) = virt_addr.kernel_to_physical() else {
			continue;
		};
		let pages = section.sh_size.div_ceil(PAGE_SIZE as _) as usize;
		transaction.map_range(phys_addr, virt_addr, pages, flags)?;
	}
	// Map VGA buffer
	#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
	transaction.map_range(
		vga::BUFFER_PHYS as _,
		vga::get_buffer_virt().into(),
		1,
		FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH | FLAG_WRITE | FLAG_GLOBAL,
	)?;
	transaction.commit();
	drop(transaction);
	kernel_vmem.bind();
	unsafe {
		OnceInit::init(&KERNEL_VMEM, Mutex::new(kernel_vmem));
	}
	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::memory::KERNEL_BEGIN;

	#[test_case]
	fn vmem_basic0() {
		let vmem = VMem::new().unwrap();
		for i in (0..0xc0000000).step_by(PAGE_SIZE) {
			assert_eq!(vmem.translate(VirtAddr(i)), None);
		}
	}

	#[test_case]
	fn vmem_basic1() {
		let vmem = VMem::new().unwrap();
		for i in (0..PHYS_MAP.memory_size).step_by(PAGE_SIZE) {
			assert_eq!(vmem.translate(KERNEL_BEGIN + i), Some(PhysAddr(i)));
		}
	}

	#[test_case]
	fn vmem_map0() {
		let mut vmem = VMem::new().unwrap();
		let mut transaction = vmem.transaction();
		transaction
			.map(PhysAddr(0x100000), VirtAddr(0x100000), 0)
			.unwrap();
		transaction.commit();
		drop(transaction);
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
		let mut vmem = VMem::new().unwrap();
		let mut transaction = vmem.transaction();
		transaction
			.map(PhysAddr(0x100000), VirtAddr(0x100000), 0)
			.unwrap();
		transaction
			.map(PhysAddr(0x200000), VirtAddr(0x100000), 0)
			.unwrap();
		transaction.commit();
		drop(transaction);
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
		let mut vmem = VMem::new().unwrap();
		let mut transaction = vmem.transaction();
		transaction
			.map(PhysAddr(0x100000), VirtAddr(0x100000), 0)
			.unwrap();
		transaction.unmap(VirtAddr(0x100000)).unwrap();
		transaction.commit();
		drop(transaction);
		for i in (0..0xc0000000).step_by(PAGE_SIZE) {
			assert_eq!(vmem.translate(VirtAddr(i)), None);
		}
	}
}
