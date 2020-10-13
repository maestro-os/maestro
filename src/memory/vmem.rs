/*
 * This file must be compiled for x86 only.
 * The virtual memory makes the kernel able to isolate processes, which is essential for modern
 * systems.
 *
 * x86 virtual memory works with a tree structure. Each element is an array of subelements. The
 * position of the elements in the arrays allows to tell the virtual address for the mapping. Under
 * 32 bits, elements are array of 32 bits long words that can contain 1024 entries. The following
 * elements are available:
 * - Page directory: The main element, contains page tables
 * - Page table: Represents a block of 4MB, each entry is a page
 *
 * Under 32 bits, pages are 4096 bytes large. Each entries of elements contains the physical
 * address to the element/page and some flags. The flags can be stored with the address in only
 * 4 bytes large entries because addresses have to be page-aligned, freeing 12 bits in the entry
 * for the flags.
 *
 * For each entries of each elements, the kernel must keep track of how many elements are being
 * used. This can be done with a simple counter: when an entry is allocated, the counter is
 * incremented and when an entry is freed, the counter is decremented. When the counter reaches 0,
 * the element can be freed.
 */

use crate::memory::Void;

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
 * The type representing a x86 page directory.
 */
type VMem = *const u32;
/*
 * Same as VMem, but mutable.
 */
type MutVMem = *mut u32;

extern "C" {
	pub fn cr0_get() -> u32;
	pub fn cr0_set(flags: u32);
	pub fn cr0_clear(flags: u32);
	pub fn cr2_get() -> u32;
	pub fn cr3_get() -> u32;

	fn paging_enable(directory: *const u32);
	fn paging_disable();
	fn tlb_reload();
}

/*
 * Structure wrapping a virtual memory. This structure contains the counter for the number of
 * elements that are used in the associated element.
 */
pub struct VMemWrapper {
	/* The number of used elements in the associated element */
	used_elements: u16,
	/* The associated element */
	vmem: VMem,
}

// TODO Find a place to store wrappers

/*
 * Initializes a new page directory. The kernel memory is mapped into the context by default.
 */
pub fn init() -> Option<VMem> {
	// TODO
	None
}

/*
 * Creates and loads the kernel's page directory. The kernel's code is protected from writing.
 */
pub fn kernel() {
	// TODO
}

/*
 * Resolves the paging entry for the given pointer. If no entry is found, None is returned. The
 * entry must be marked as present to be found. If Page Size Extention (PSE) is used, an entry of
 * the page directory might be returned.
 */
pub fn resolve(_vmem: VMem, _ptr: *const Void) -> Option<*const u32> {
	// TODO
	None
}

/*
 * Tells whether the given pointer `ptr` is mapped or not.
 */
pub fn is_mapped(_vmem: VMem, _ptr: *const Void) -> bool {
	// TODO
	false
}

/*
 * Checks if the portion of memory beginning at `ptr` with size `size` is mapped.
 */
pub fn contains(_vmem: VMem, _ptr: *const Void, _size: usize) -> bool {
	// TODO
	false
}

/*
 * Translates the given virtual address `ptr` to the corresponding physical address. If the address
 * is not mapped, None is returned.
 */
pub fn translate(_vmem: VMem, _ptr: *const Void) -> Option<*const Void> {
	// TODO
	None
}

/*
 * Resolves the entry for the given virtual address `ptr` and returns its flags. This function
 * might return a page directory entry if a large block is present at the corresponding location.
 * If no entry is found, the function returns None.
 */
pub fn get_flags(_vmem: VMem, _ptr: *const Void) -> Option<u32> {
	// TODO
	None
}

/*
 * Maps the the given physical address `physaddr` to the given virtual address `virtaddr` with the
 * given flags. The function forces the FLAG_PAGE_PRESENT flag.
 */
pub fn map(_vmem: VMem, _physaddr: *const Void, _virtaddr: *const Void, _flags: u32) {
	// TODO
}

/*
 * Maps the given physical address `physaddr` to the given virtual address `virtaddr` with the
 * given flags using blocks of 1024 pages (PSE).
 */
pub fn map_pse(_vmem: VMem, _physaddr: *const Void, _virtaddr: *const Void, _flags: u32) {
	// TODO
}

/*
 * Maps the given range of physical address `physaddr` to the given range of virtual address
 * `virtaddr`. The range is `pages` pages large.
 */
pub fn map_range(_vmem: VMem, _physaddr: *const Void, _virtaddr: *const Void, _pages: usize,
	_flags: u32) {
	// TODO
}

/*
 * Maps the physical address `ptr` to the same address in virtual memory with the given flags
 * `flags`.
 */
pub fn identity(_vmem: VMem, _ptr: *const Void, _flags: u32) {
	// TODO
}

/*
 * Maps the physical address `ptr` to the same address in virtual memory with the given flags
 * `flags`, using blocks of 1024 pages (PSE).
 */
pub fn identity_pse(_vmem: VMem, _ptr: *const Void, _flags: u32) {
	// TODO
}

/*
 * Identity maps a range beginning at physical address `from` with pages `pages` and flags `flags`.
 */
pub fn identity_range(_vmem: VMem, _from: *const Void, _pages: usize, _flags: u32) {
	// TODO
}

/*
 * Unmaps the page at virtual address `virtaddr`. The function unmaps only one page, thus if a
 * large block is present at this location (PSE), it shall be split down into a table which shall
 * be filled accordingly.
 */
pub fn unmap(_vmem: VMem, _virtaddr: *const Void) {
	// TODO
}

/*
 * Unmaps the given range beginning at virtual address `virtaddr` with size of `pages` pages. Large
 * blocks splitting is supported (PSE).
 */
pub fn unmap_range(_vmem: VMem, _virtaddr: *const Void, _pages: usize) {
	// TODO
}

/*
 * Clones the given page directory, allocating copies of every children elements. If the page
 * directory cannot be cloned, the function returns None.
 */
pub fn clone(_vmem: VMem) -> Option<VMem> {
	// TODO
	None
}

/*
 * Flushes the modifications of the given page directory by reloading the Translation Lookaside
 * Buffer (TLB).
 */
pub fn flush(_vmem: VMem) {
	// TODO
}

/*
 * Destroyes the given page directory, including its children elements. If the page directory is
 * begin used, the behaviour is undefined.
 */
pub fn destroy(_vmem: VMem) {
	// TODO
}
