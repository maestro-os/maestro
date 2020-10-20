/*
 * TODO Documentation
 */

pub mod buddy;
pub mod memmap;
pub mod vmem;

/*
 * Used to represent the `void` keyword in `void *` in C.
 */
pub type Void = u8;

/*
 * Null pointer of type *const Void.
 */
pub const NULL: *const Void = 0x0 as _;

/*
 * The size of a page in bytes.
 */
pub const PAGE_SIZE: usize = 0x1000;

/*
 * The physical pointer to the beginning of the kernel.
 */
pub const KERNEL_PHYS_BEGIN: *const Void = 0x100000 as *const _;
/*
 * The pointer to the end of the virtual memory reserved to the process.
 */
pub const PROCESS_END: *const Void = 0xc0000000 as *const _;

/*
 * Returns a pointer to the beginning of the kernel in the virtual address space.
 */
#[inline(always)]
pub fn get_kernel_virtual_begin() -> *const Void {
	unsafe {
		&kernel_begin as *const _
	}
}

/*
 * Returns the size of the kernel image in bytes.
 */
#[inline(always)]
pub fn get_kernel_size() -> usize {
	unsafe {
		(&kernel_end as *const _ as usize) - (&kernel_begin as *const _ as usize)
	}
}

/*
 * Returns the end of the kernel image in the physical memory.
 */
#[inline(always)]
pub fn get_kernel_end() -> *const Void {
	unsafe {
		KERNEL_PHYS_BEGIN.offset(get_kernel_size() as isize)
	}
}

/*
 * Returns the end of the kernel image in the virtual memory.
 */
#[inline(always)]
pub fn get_kernel_virtual_end() -> *const Void {
	unsafe {
		get_kernel_virtual_begin().offset(get_kernel_size() as isize)
	}
}

/*
 * Converts the page number to a pointer to the beginning of the pages.
 */
pub fn page_to_ptr(page: usize) -> *const Void {
	(page * PAGE_SIZE) as *const _
}

/*
 * Converts a pointer to the page index containing it.
 */
pub fn ptr_to_page(ptr: *const Void) -> usize {
	(ptr as usize) / PAGE_SIZE
}

/*
 * Gives the table index for the given address.
 */
pub fn addr_table(addr: *const Void) -> usize {
	((addr as usize) >> 22) & 0x3ff
}

/*
 * Gives the page index for the given address.
 */
pub fn addr_page(addr: *const Void) -> usize {
	((addr as usize) >> 12) & 0x3ff
}

/*
 * Gives the offset of the pointer in its page.
 */
pub fn addr_remain(addr: *const Void) -> usize {
	(addr as usize) & 0xfff
}

/*
 * Converts a kernel physical address to a virtual address.
 */
pub fn kern_to_virt(ptr: *const Void) -> *const Void {
	((ptr as usize) + (PROCESS_END as usize)) as *const _
}

/*
 * Converts a kernel virtual address to a physical address.
 */
pub fn kern_to_phys(ptr: *const Void) -> *const Void {
	((ptr as usize) - (PROCESS_END as usize)) as *const _
}

/*
 * Symbols to the beginning and the end of the kernel.
 */
extern "C" {
	static kernel_begin: Void;
	static kernel_end: Void;
}
