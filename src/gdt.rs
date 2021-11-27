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
    /// Returns the entry's base address.
    #[inline(always)]
    pub fn get_base(&self) -> u32 {
        ((self.val >> 16 & 0xffffff) | (self.val >> 56 & 0xff)) as _
    }

    /// Sets the entry's base address.
    #[inline(always)]
    pub fn set_base(&mut self, base: u32) {
        self.val &= !(0xffffff << 16);
        self.val &= !(0xff << 56);

        self.val |= (base as u64 & 0xffffff) << 16;
        self.val |= ((base as u64 >> 24) & 0xff) << 56;
    }

    /// Returns the entry's limit.
    #[inline(always)]
    pub fn get_limit(&self) -> u32 {
        ((self.val & 0xffff) | (self.val >> 48 & 0xf)) as _
    }

    /// Sets the entry's limit. If the given limit is more than (2^20 - 1), the value is truncated.
    #[inline(always)]
    pub fn set_limit(&mut self, limit: u32) {
        self.val &= !0xffff;
        self.val &= !(0xf << 48);

        self.val |= limit as u64 & 0xffff;
        self.val |= ((limit as u64 >> 16) & 0xf) << 48;
    }

	/// Tells whether the entry is present.
	#[inline(always)]
	pub fn is_present(&self) -> bool {
		(self.val >> 47 & 1) != 0
	}

	/// Sets the entry present or not.
	#[inline(always)]
	pub fn set_present(&mut self, present: bool) {
	    if present {
		    self.val |= 1 << 47;
	    } else {
		    self.val &= !(1 << 47);
	    }
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
