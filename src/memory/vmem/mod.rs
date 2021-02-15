/// The virtual memory makes the kernel able to isolate processes, which is essential for modern
/// systems.

// TODO Make this file fully cross-platform

// TODO Only if on the corresponding architecture
pub mod x86;

use core::ffi::c_void;
use crate::memory;
use crate::util::boxed::Box;

/// Trait representing virtual memory context handler. This trait is the interface to manipulate
/// virtual memory on any architecture. Each architecture has its own structure implementing this
/// trait.
pub trait VMem: Drop {
	// TODO doc
	fn is_mapped(&self, ptr: *const c_void) -> bool;
	// TODO doc
	fn translate(&self, ptr: *const c_void) -> Option<*const c_void>;

	// TODO doc
	fn map(&mut self, physaddr: *const c_void, virtaddr: *const c_void, flags: u32)
		-> Result<(), ()>;
	// TODO doc
	fn map_range(&mut self, physaddr: *const c_void, virtaddr: *const c_void, pages: usize,
		flags: u32) -> Result<(), ()>;

	/// Maps the physical address `ptr` to the same address in virtual memory with the given flags
	/// `flags`.
	fn identity(&mut self, ptr: *const c_void, flags: u32) -> Result<(), ()> {
		self.map(ptr, ptr, flags)
	}
	/// Identity maps a range beginning at physical address `from` with pages `pages` and flags
	/// `flags`.
	fn identity_range(&mut self, ptr: *const c_void, pages: usize, flags: u32) -> Result<(), ()> {
		self.map_range(ptr, ptr, pages, flags)
	}

	// TODO doc
	fn unmap(&mut self, virtaddr: *const c_void) -> Result<(), ()>;
	// TODO doc
	fn unmap_range(&mut self, virtaddr: *const c_void, pages: usize) -> Result<(), ()>;

	fn clone(&self) -> Result::<Self, ()> where Self: Sized;

	// TODO doc
	fn flush(&self);
}

/// Creates a new virtual memory context handler for the current architecture.
pub fn new() -> Result::<Box::<dyn VMem>, ()> {
	Box::new(x86::X86VMem::new()?)
}

// TODO Handle leak
/// Creates and loads the kernel's memory protection, protecting its code from writing.
pub fn kernel() {
	if let Ok(kernel_vmem) = new() {
		unsafe {
			x86::paging_enable(memory::kern_to_phys(kernel_vmem as _) as _);
		}
	} else {
		crate::kernel_panic!("Cannot initialize kernel virtual memory!", 0);
	}
}

/// Tells whether the read-only pages protection is enabled.
pub fn is_write_lock() -> bool {
	unsafe {
		(x86::cr0_get() & (1 << 16)) != 0
	}
}

/// Sets whether the kernel can write to read-only pages.
pub fn set_write_lock(lock: bool) {
	if lock {
		unsafe {
			x86::cr0_set(1 << 16);
		}
	} else {
		unsafe {
			x86::cr0_clear(1 << 16);
		}
	}
}

/// Executes the closure given as parameter. During execution, the kernel can write on read-only
/// pages. The state of the write lock is restored after the closure's execution.
pub unsafe fn write_lock_wrap<T: Fn()>(f: T) {
	let lock = is_write_lock();
	set_write_lock(false);

	f();

	set_write_lock(lock);
}
