//! Under the x86 architecture, the GDT (Global Descriptior Table) is a table of structure that
//! describes the segments of memory. It is a deprecated structure that still must be used in order
//! to switch to protected mode, handle protection rings and load the Task State Segment (TSS).

use core::ffi::c_void;
use crate::memory;

/// The address in physical memory to the beginning of the GDT.
const PHYS_PTR: *mut c_void = 0x800 as _;

/// The offset of the kernel code segment.
pub const KERNEL_CS: u32 = 8;
/// The offset of the kernel data segment.
pub const KERNEL_DS: u32 = 16;
/// The offset of the user code segment.
pub const USER_CS: u32 = 24;
/// The offset of the user data segment.
pub const USER_DS: u32 = 32;
/// The offset of the Task State Segment (TSS).
pub const TSS_OFFSET: u32 = 48;
/// The offset of TLS (Thread Local Storage) entries.
pub const TLS_OFFSET: u32 = 56;

/// Structure representing a GDT entry.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Entry {
	/// The entry's value.
	val: u64,
}

impl Entry {
	/// Tells whether the entry is present.
	#[inline(always)]
	pub fn is_present(&self) -> bool {
		(self.val >> 47 & 1) != 0
	}
}

impl Default for Entry {
	fn default() -> Self {
		Self {
			val: 0,
		}
	}
}

/// x86. Creates a segment selector for the given segment offset and ring.
#[inline(always)]
pub fn make_segment_selector(offset: u32, ring: u32) -> u16 {
	debug_assert!(ring <= 3);
	(offset | ring) as _
}

/// x86. Returns the pointer to the segment at offset `offset`.
pub fn get_segment_ptr(offset: u32) -> *mut u64 {
	unsafe {
		memory::kern_to_virt(PHYS_PTR.add(offset as _)) as _
	}
}
