/// This file contains some constants used everywhere in the kernel.

use core::ffi::c_void;

/// The size of a page in bytes.
pub const PAGE_SIZE: usize = 0x1000;

/// The physical pointer to the beginning of the kernel.
pub const KERNEL_PHYS_BEGIN: *const c_void = 0x100000 as *const _;
/// The pointer to the end of the virtual memory reserved to the process.
pub const PROCESS_END: *const c_void = 0xc0000000 as *const _;
