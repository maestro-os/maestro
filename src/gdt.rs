/// TODO doc

use core::ffi::c_void;
use crate::memory;

/// TODO doc
const PHYS_PTR: *mut c_void = 0x800 as _;

/// TODO doc
pub const KERNEL_CODE_OFFSET: usize = 8;
/// TODO doc
pub const KERNEL_DATA_OFFSET: usize = 16;
/// TODO doc
pub const USER_CODE_OFFSET: usize = 24;
/// TODO doc
pub const USER_DATA_OFFSET: usize = 32;
/// TODO doc
pub const TSS_OFFSET: usize = 40;

/// x86. Creates a segment selector for the given segment offset and ring.
#[inline(always)]
pub fn make_segment_selector(offset: usize, ring: usize) -> u16 {
	debug_assert!(ring <= 3);
	(offset | ring) as _
}

/// x86. Returns the pointer to the segment at offset `offset`.
pub fn get_segment_ptr(offset: usize) -> *mut u64 {
	unsafe { // Pointer arithmetic
		memory::kern_to_virt(PHYS_PTR.add(offset)) as _
	}
}
