//! The virtual memory makes the kernel able to isolate processes, which is
//! essential for modern systems.

// TODO Make this file fully cross-platform

#[cfg(target_arch = "x86")]
pub mod x86;

use crate::{
	elf,
	errno::{AllocError, AllocResult},
	idt, memory, register_get, register_set,
	util::{boxed::Box, TryClone},
};
use core::ffi::c_void;

/// Trait representing virtual memory context handler.
///
/// This trait is the interface to manipulate virtual memory on any architecture.
///
/// Each architecture has its own structure implementing this trait.
///
/// Virtual memory contexts use interior mutability.
pub trait VMem: TryClone<Error = AllocError> {
	/// Translates the given virtual address `ptr` to the corresponding physical
	/// address.
	///
	/// If the address is not mapped, the function returns `None`.
	fn translate(&self, ptr: *const c_void) -> Option<*const c_void>;

	/// Tells whether the given pointer `ptr` is mapped or not.
	fn is_mapped(&self, ptr: *const c_void) -> bool {
		self.translate(ptr).is_some()
	}

	/// Maps the the given physical address `physaddr` to the given virtual
	/// address `virtaddr` with the given flags.
	///
	/// This function automatically invalidates the page in the cache.
	///
	/// # Safety
	///
	/// If the context is bound, the caller must ensure that regions of memory to be used by the
	/// execution context are left valid.
	unsafe fn map(
		&self,
		physaddr: *const c_void,
		virtaddr: *const c_void,
		flags: u32,
	) -> AllocResult<()>;
	/// Maps the given range of physical address `physaddr` to the given range
	/// of virtual address `virtaddr`.
	///
	/// The range is `pages` pages large.
	///
	/// If the operation fails, the virtual memory is left altered midway.
	///
	/// This function automatically invalidates the page(s) in the cache.
	///
	/// # Safety
	///
	/// If the context is bound, the caller must ensure that regions of memory to be used by the
	/// execution context are left valid.
	unsafe fn map_range(
		&self,
		physaddr: *const c_void,
		virtaddr: *const c_void,
		pages: usize,
		flags: u32,
	) -> AllocResult<()>;

	/// Unmaps the page at virtual address `virtaddr`.
	///
	/// This function automatically invalidates the page in the cache.
	///
	/// # Safety
	///
	/// If the context is bound, the caller must ensure that regions of memory to be used by the
	/// execution context are left valid.
	unsafe fn unmap(&self, virtaddr: *const c_void) -> AllocResult<()>;
	/// Unmaps the given range beginning at virtual address `virtaddr` with size
	/// of `pages` pages.
	///
	/// If the operation fails, the virtual memory is left altered midway.
	///
	/// This function automatically invalidates the page(s) in the cache.
	///
	/// # Safety
	///
	/// If the context is bound, the caller must ensure that regions of memory to be used by the
	/// execution context are left valid.
	unsafe fn unmap_range(&self, virtaddr: *const c_void, pages: usize) -> AllocResult<()>;

	/// Binds the virtual memory context handler.
	///
	/// # Safety
	///
	/// This function totally breaks Rust's safety guarantee.
	/// The caller must ensure the stack, code and data the code might access are still accessible
	/// in the memory context.
	unsafe fn bind(&self);
	/// Tells whether the handler is bound or not.
	fn is_bound(&self) -> bool;

	/// Invalidates the page at address `addr` from the CPU's cache.
	fn invalidate_page(&self, addr: *const c_void);
	/// Flushes the modifications of the context if bound.
	///
	/// This function should be called after applying modifications to the context for them to be
	/// taken into account.
	///
	/// This is an expensive operation for the CPU cache and should be used as few as possible.
	fn flush(&self);

	/// Protects the kernel's read-only sections from writing.
	fn protect_kernel(&self) -> AllocResult<()> {
		let iter = elf::kernel::sections().filter(|s| {
			s.sh_flags & elf::SHF_WRITE == 0 && s.sh_addralign as usize == memory::PAGE_SIZE
		});
		for section in iter {
			let phys_addr = memory::kern_to_phys(section.sh_addr as _);
			let virt_addr = memory::kern_to_virt(section.sh_addr as _);
			let pages = section.sh_size.div_ceil(memory::PAGE_SIZE as _) as usize;
			unsafe {
				self.map_range(phys_addr, virt_addr, pages, x86::FLAG_USER)?;
			}
		}
		Ok(())
	}
}

/// Creates a new virtual memory context handler for the current architecture.
pub fn new() -> AllocResult<Box<dyn VMem>> {
	Ok(Box::new(x86::X86VMem::new()?)? as Box<dyn VMem>)
}

/// Clones the virtual memory context handler `vmem`.
pub fn try_clone(vmem: &dyn VMem) -> AllocResult<Box<dyn VMem>> {
	let vmem = unsafe { &*(vmem as *const dyn VMem as *const x86::X86VMem) };
	Ok(Box::new(vmem.try_clone()?)? as Box<dyn VMem>)
}

/// Tells whether the read-only pages protection is enabled.
pub fn is_write_locked() -> bool {
	unsafe { (register_get!("cr0") & (1 << 16)) != 0 }
}

/// Sets whether the kernel can write to read-only pages.
///
/// # Safety
///
/// This function disables memory protection on the kernel side, which makes
/// read-only data writable.
///
/// Writing on read-only regions of memory has an undefined behavior.
pub unsafe fn set_write_lock(lock: bool) {
	let mut val = register_get!("cr0");
	if lock {
		val |= 1 << 16;
	} else {
		val &= !(1 << 16);
	}
	register_set!("cr0", val);
}

/// Executes the closure given as parameter.
///
/// During execution, the kernel can write on read-only pages.
///
/// The state of the write lock is restored after the closure's execution.
///
/// # Safety
///
/// This function disables memory protection on the kernel side, which makes
/// read-only data writable.
///
/// Writing on read-only regions of memory has an undefined behavior.
pub unsafe fn write_lock_wrap<F: FnOnce() -> T, T>(f: F) -> T {
	let lock = is_write_locked();
	set_write_lock(false);
	let result = f();
	set_write_lock(lock);
	result
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
/// Special consideration should be taken when using this function since Rust is
/// unable to ensure its safety.
///
/// The caller must ensure that the stack is accessible in both the current and given virtual
/// memory contexts.
///
/// If the closure changes the current memory context, the behaviour is
/// undefined.
pub unsafe fn switch<F: FnOnce() -> T, T>(vmem: &dyn VMem, f: F) -> T {
	idt::wrap_disable_interrupts(|| {
		if vmem.is_bound() {
			f()
		} else {
			// Get current vmem
			let cr3 = register_get!("cr3");
			// Bind temporary vmem
			vmem.bind();

			let result = f();

			// Restore previous vmem
			x86::enable_paging(cr3 as _);

			result
		}
	})
}

#[cfg(test)]
mod test {
	use super::*;
	use crate::memory;

	#[test_case]
	fn vmem_basic0() {
		let vmem = new().unwrap();
		for i in (0usize..0xc0000000).step_by(memory::PAGE_SIZE) {
			assert_eq!(vmem.translate(i as _), None);
		}
	}

	#[test_case]
	fn vmem_basic1() {
		let vmem = new().unwrap();
		for i in (0..0x40000000).step_by(memory::PAGE_SIZE) {
			let virt_ptr = ((memory::PROCESS_END as usize) + i) as _;
			let result = vmem.translate(virt_ptr);
			assert_ne!(result, None);
			let phys_ptr = result.unwrap();
			assert_eq!(phys_ptr, i as _);
		}
	}

	#[test_case]
	fn vmem_map0() {
		let vmem = new().unwrap();
		unsafe {
			vmem.map(0x100000 as _, 0x100000 as _, 0).unwrap();
		}

		for i in (0usize..0xc0000000).step_by(memory::PAGE_SIZE) {
			if i >= 0x100000 && i < 0x101000 {
				let result = vmem.translate(i as _);
				assert!(result.is_some());
				assert_eq!(result.unwrap(), i as _);
			} else {
				assert_eq!(vmem.translate(i as _), None);
			}
		}
	}

	#[test_case]
	fn vmem_map1() {
		let vmem = new().unwrap();
		unsafe {
			vmem.map(0x100000 as _, 0x100000 as _, 0).unwrap();
			vmem.map(0x200000 as _, 0x100000 as _, 0).unwrap();
		}

		for i in (0usize..0xc0000000).step_by(memory::PAGE_SIZE) {
			if i >= 0x100000 && i < 0x101000 {
				let result = vmem.translate(i as _);
				assert!(result.is_some());
				assert_eq!(result.unwrap(), (0x100000 + i) as _);
			} else {
				assert_eq!(vmem.translate(i as _), None);
			}
		}
	}

	// TODO More tests on map
	// TODO Test on map_range

	#[test_case]
	fn vmem_unmap0() {
		let vmem = new().unwrap();
		unsafe {
			vmem.map(0x100000 as _, 0x100000 as _, 0).unwrap();
			vmem.unmap(0x100000 as _).unwrap();
		}

		for i in (0usize..0xc0000000).step_by(memory::PAGE_SIZE) {
			assert_eq!(vmem.translate(i as _), None);
		}
	}

	// TODO More tests on unmap
	// TODO Test on unmap_range

	// TODO Add more tests
}
