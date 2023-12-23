//! Under the x86 architecture, the GDT (Global Descriptior Table) is a table of
//! structure that describes the segments of memory.
//!
//! It is a deprecated structure that still must be used in order to switch to protected mode,
//! handle protection rings and load the Task State Segment (TSS).

use crate::memory;
use core::arch::asm;
use core::ffi::c_void;
use core::fmt;
use core::ptr;

/// The address in physical memory to the beginning of the GDT.
const PHYS_PTR: *mut c_void = 0x800 as _;

/// The offset of the kernel code segment.
pub const KERNEL_CS: usize = 8;
/// The offset of the kernel data segment.
pub const KERNEL_DS: usize = 16;
/// The offset of the user code segment.
pub const USER_CS: usize = 24;
/// The offset of the user data segment.
pub const USER_DS: usize = 32;
/// The offset of the Task State Segment (TSS).
pub const TSS_OFFSET: usize = 40;
/// The offset of Thread Local Storage (TLS) entries.
pub const TLS_OFFSET: usize = 48;

/// Structure representing a GDT entry.
#[repr(transparent)]
#[derive(Clone, Copy, Default)]
pub struct Entry(pub u64);

impl Entry {
	/// Returns the entry's base address.
	#[inline(always)]
	pub fn get_base(&self) -> u32 {
		(((self.0 >> 16) & 0xffffff) | ((self.0 >> 32) & 0xff000000)) as _
	}

	/// Sets the entry's base address.
	#[inline(always)]
	pub fn set_base(&mut self, base: u32) {
		self.0 &= !(0xffffff << 16);
		self.0 &= !(0xff << 56);

		self.0 |= (base as u64 & 0xffffff) << 16;
		self.0 |= ((base as u64 >> 24) & 0xff) << 56;
	}

	/// Returns the entry's limit.
	#[inline(always)]
	pub fn get_limit(&self) -> u32 {
		((self.0 & 0xffff) | (((self.0 >> 48) & 0xf) << 16)) as _
	}

	/// Sets the entry's limit.
	///
	/// If the given limit is more than `pow(2, 20) - 1`, the value is truncated.
	#[inline(always)]
	pub fn set_limit(&mut self, limit: u32) {
		self.0 &= !0xffff;
		self.0 &= !(0xf << 48);

		self.0 |= limit as u64 & 0xffff;
		self.0 |= ((limit as u64 >> 16) & 0xf) << 48;
	}

	/// Returns the value of the access byte.
	#[inline(always)]
	pub fn get_access_byte(&self) -> u8 {
		((self.0 >> 40) & 0xff) as _
	}

	/// Sets the value of the access byte.
	#[inline(always)]
	pub fn set_access_byte(&mut self, byte: u8) {
		self.0 &= !(0xff << 40);
		self.0 |= (byte as u64) << 40;
	}

	/// Returns the flags.
	#[inline(always)]
	pub fn get_flags(&self) -> u8 {
		((self.0 >> 52) & 0x0f) as _
	}

	/// Sets the flags.
	#[inline(always)]
	pub fn set_flags(&mut self, flags: u8) {
		self.0 &= !(0x0f << 52);
		self.0 |= ((flags as u64) & 0x0f) << 52;
	}

	/// Tells whether the entry is present.
	#[inline(always)]
	pub fn is_present(&self) -> bool {
		(self.0 >> 47 & 1) != 0
	}

	/// Sets the entry present or not.
	#[inline(always)]
	pub fn set_present(&mut self, present: bool) {
		if present {
			self.0 |= 1 << 47;
		} else {
			self.0 &= !(1 << 47);
		}
	}

	/// Updates the entry at offset `off` of the GDT with the current entry.
	///
	/// # Safety
	///
	/// An invalid offset, either not a multiple of 8 or out of bounds of the GDT, shall result in
	/// an undefined behaviour.
	pub unsafe fn update_gdt(&self, off: usize) {
		let ptr = get_segment_ptr(off);
		ptr::write_volatile(ptr, self.0);
	}
}

impl fmt::Display for Entry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		writeln!(f, "Descriptor Table Entry:")?;
		writeln!(f, "base: {:x}", self.get_base())?;
		writeln!(f, "limit: {:x}", self.get_limit())?;
		writeln!(f, "access byte: {:x}", self.get_access_byte())?;
		writeln!(f, "flags: {:x}", self.get_flags())?;
		writeln!(f, "present: {}", self.is_present())
	}
}

/// Creates a segment selector for the given segment offset and ring.
#[inline(always)]
pub fn make_segment_selector(offset: u32, ring: u32) -> u16 {
	debug_assert!(ring <= 3);
	(offset | ring) as _
}

/// Returns the pointer to the segment at offset `offset`.
pub fn get_segment_ptr(offset: usize) -> *mut u64 {
	unsafe { memory::kern_to_virt(PHYS_PTR.add(offset as _)) as _ }
}

/// Refreshes the GDT's cache.
#[inline(always)]
pub fn flush() {
	unsafe {
		asm!("lgdt GDT_DESC_VIRT_PTR");
	}
}
