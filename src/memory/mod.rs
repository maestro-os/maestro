/// TODO Documentation

pub mod buddy;
pub mod malloc;
pub mod memmap;
pub mod vmem;

use core::ffi::c_void;

/// Null pointer of type *const c_void.
pub const NULL: *const c_void = 0x0 as _;

/// The size of a page in bytes.
pub const PAGE_SIZE: usize = 0x1000;

/// The physical pointer to the beginning of the kernel.
pub const KERNEL_PHYS_BEGIN: *const c_void = 0x100000 as *const _;
/// The pointer to the end of the virtual memory reserved to the process.
pub const PROCESS_END: *const c_void = 0xc0000000 as *const _;

/// Returns a pointer to the beginning of the kernel in the virtual address space.
#[inline(always)]
pub fn get_kernel_virtual_begin() -> *const c_void {
	unsafe {
		&kernel_begin as *const _
	}
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
		((PROCESS_END as usize) + (&kernel_end as *const _ as usize)) as _
	}
}

/// Converts the page number to a pointer to the beginning of the pages.
pub fn page_to_ptr(page: usize) -> *const c_void {
	(page * PAGE_SIZE) as *const _
}

/// Converts a pointer to the page index containing it.
pub fn ptr_to_page(ptr: *const c_void) -> usize {
	(ptr as usize) / PAGE_SIZE
}

/// Gives the table index for the given address.
pub fn addr_table(addr: *const c_void) -> usize {
	((addr as usize) >> 22) & 0x3ff
}

/// Gives the page index for the given address.
pub fn addr_page(addr: *const c_void) -> usize {
	((addr as usize) >> 12) & 0x3ff
}

/// Gives the offset of the pointer in its page.
pub fn addr_remain(addr: *const c_void) -> usize {
	(addr as usize) & 0xfff
}

/// Converts a kernel physical address to a virtual address.
pub fn kern_to_virt(ptr: *const c_void) -> *const c_void {
	debug_assert!(ptr < PROCESS_END);
	((ptr as usize) + (PROCESS_END as usize)) as *const _
}

/// Converts a kernel virtual address to a physical address.
pub fn kern_to_phys(ptr: *const c_void) -> *const c_void {
	debug_assert!(ptr >= PROCESS_END);
	((ptr as usize) - (PROCESS_END as usize)) as *const _
}

/// Symbols to the beginning and the end of the kernel.
extern "C" {
	static kernel_begin: c_void;
	static kernel_end: c_void;
}
