//! The memory is one of the main component of the system.
//! This module handles almost every memory-related features, including physical memory map
//! retrieving, memory allocation, virtual memory management, ...

pub mod alloc;
pub mod buddy;
pub mod malloc;
pub mod memmap;
pub mod mmio;
pub mod stack;
pub mod vmem;

use core::ffi::c_void;

/// The size of a page in bytes.
pub const PAGE_SIZE: usize = 0x1000;

/// The physical pointer to the beginning of the kernel.
pub const KERNEL_PHYS_BEGIN: *const c_void = 0x100000 as *const _;

/// Pointer to the beginning of the allocatable region in the virtual memory.
pub const ALLOC_BEGIN: *const c_void = 0x40000000 as *const _;
/// Pointer to the end of the virtual memory reserved to the process.
pub const PROCESS_END: *const c_void = 0xc0000000 as *const _;

/// Symbols to the beginning and the end of the kernel.
extern "C" {
	/// The kernel begin symbol, giving the pointer to the begin of the kernel image in the virtual
	/// memory. This memory location should never be accessed using this symbol.
	static kernel_begin: c_void;
	/// The kernel end symbol, giving the pointer to the end of the kernel image in the virtual
	/// memory. This memory location should never be accessed using this symbol.
	static kernel_end: c_void;
}

/// Returns a pointer to the beginning of the kernel in the virtual address space.
#[inline(always)]
pub fn get_kernel_virtual_begin() -> *const c_void {
	unsafe {
		&kernel_begin as *const _
	}
}

/// The size of the kernelspace memory in bytes.
#[inline(always)]
pub fn get_kernelspace_size() -> usize {
	usize::MAX - PROCESS_END as usize + 1
}

/// Returns the size of the kernel image in bytes.
#[inline(always)]
pub fn get_kernel_size() -> usize {
	unsafe {
		(&kernel_end as *const _ as usize) - (&kernel_begin as *const _ as usize)
	}
}

/// Returns the end of the kernel image in the physical memory.
#[inline(always)]
pub fn get_kernel_end() -> *const c_void {
	unsafe {
		((&kernel_end as *const c_void as usize) - (PROCESS_END as usize)) as _
	}
}

/// Returns the end of the kernel image in the virtual memory.
#[inline(always)]
pub fn get_kernel_virtual_end() -> *const c_void {
	unsafe {
		(&kernel_end as *const _ as usize) as _
	}
}

/// Converts a kernel physical address to a virtual address.
pub fn kern_to_virt(ptr: *const c_void) -> *const c_void {
	if (ptr as usize) < get_kernelspace_size() {
		((ptr as usize) + (PROCESS_END as usize)) as *const _
	} else {
		ptr
	}
}

/// Converts a kernel virtual address to a physical address.
pub fn kern_to_phys(ptr: *const c_void) -> *const c_void {
	if ptr as usize >= PROCESS_END as usize {
		((ptr as usize) - (PROCESS_END as usize)) as *const _
	} else {
		ptr
	}
}
