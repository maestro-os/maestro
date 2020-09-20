/*
 * TODO Documentation
 */

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
 * x86 paging flag. If set, pages are 4 MB long.
 */
pub const PAGING_TABLE_PAGE_SIZE: u32 = 0b10000000;
/*
 * x86 paging flag. Set if the page has been read or wrote.
 */
pub const PAGING_TABLE_ACCESSED: u32 = 0b00100000;
/*
 * x86 paging flag. If set, page will not be cached.
 */
pub const PAGING_TABLE_CACHE_DISABLE: u32 = 0b00010000;
/*
 * x86 paging flag. If set, write-through caching is enabled.
 * If not, then write-back is enabled instead.
 */
pub const PAGING_TABLE_WRITE_THROUGH: u32 = 0b00001000;
/*
 * x86 paging flag. If set, the page can be accessed by userspace operations.
 */
pub const PAGING_TABLE_USER: u32 = 0b00000100;
/*
 * x86 paging flag. If set, the page can be wrote.
 */
pub const PAGING_TABLE_WRITE: u32 = 0b00000010;
/*
 * x86 paging flag. If set, the page is present.
 */
pub const PAGING_TABLE_PRESENT: u32 = 0b00000001;

pub const PAGING_PAGE_GLOBAL: u32 = 0b100000000;
pub const PAGING_PAGE_DIRTY: u32 = 0b001000000;
pub const PAGING_PAGE_ACCESSED: u32 = 0b000100000;
pub const PAGING_PAGE_CACHE_DISABLE: u32 = 0b000010000;
pub const PAGING_PAGE_WRITE_THROUGH: u32 = 0b000001000;
pub const PAGING_PAGE_USER: u32 = 0b000000100;
pub const PAGING_PAGE_WRITE: u32 = 0b000000010;
pub const PAGING_PAGE_PRESENT: u32 = 0b000000001;

/*
 * Flags mask in a page directory entry.
 */
pub const PAGING_FLAGS_MASK: u32 = 0xfff;
/*
 * Address mask in a page directory entry. The address doesn't need every bytes
 * since it must be page-aligned.
 */
pub const PAGING_ADDR_MASK: u32 = !PAGING_FLAGS_MASK;

/*
 * x86 page fault flag. If set, the page was present.
 */
pub const PAGE_FAULT_PRESENT: u32 = 0b00001;
/*
 * x86 page fault flag. If set, the error was caused bt a write operation, else
 * the error was caused by a read operation.
 */
pub const PAGE_FAULT_WRITE: u32 = 0b00010;
/*
 * x86 page fault flag. If set, the page fault was caused by a userspace
 * operation.
 */
pub const PAGE_FAULT_USER: u32 = 0b00100;
/*
 * x86 page fault flag. If set, one or more page directory entries contain
 * reserved bits which are set.
 */
pub const PAGE_FAULT_RESERVED: u32 = 0b01000;
/*
 * x86 page fault flag. If set, the page fault was caused by an instruction
 * fetch.
 */
pub const PAGE_FAULT_INSTRUCTION: u32 = 0b10000;

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
	((PROCESS_END as usize) + (ptr as usize)) as *const _
}

/*
 * Symbols to the beginning and the end of the kernel.
 */
extern "C" {
	static kernel_begin: Void;
	static kernel_end: Void;
}
