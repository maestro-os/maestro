/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Under the x86 architecture, the GDT (Global Descriptor Table) is a table of
//! structure that describes the segments of memory.
//!
//! It is a deprecated structure that still must be used in order to switch to protected mode,
//! handle protection rings and load the Task State Segment (TSS).

use crate::{memory::VirtAddr, process::scheduler::per_cpu};
use core::{arch::asm, cell::UnsafeCell, fmt};

/// The offset of the kernel code segment.
pub const KERNEL_CS: usize = 8;
/// The offset of the kernel data segment.
pub const KERNEL_DS: usize = 16;
/// The offset of the user code segment.
pub const USER_CS: usize = 24;
/// The offset of the user data segment (32 bits).
pub const USER_DS: usize = 32;
/// The offset of the user data segment (64 bits).
pub const USER_CS64: usize = 40;
/// The offset of the Task State Segment (TSS).
pub const TSS_OFFSET: usize = 48;
/// The offset of Thread Local Storage (TLS) entries.
pub const TLS_OFFSET: usize = 64;

/// A GDT entry.
#[repr(C, align(8))]
#[derive(Clone, Copy, Default)]
pub struct Entry(pub u64);

impl Entry {
	/// Creates a new entry with the give information.
	#[inline(always)]
	pub const fn new(base: u32, limit: u32, access_byte: u8, flags: u8) -> Self {
		let mut ent = Self(0);
		ent.set_base(base);
		ent.set_limit(limit);
		ent.set_access_byte(access_byte);
		ent.set_flags(flags);
		ent
	}

	/// Creates a long mode entry, spanning two regular entries.
	pub const fn new64(base: u64, limit: u32, access_byte: u8, flags: u8) -> [Self; 2] {
		[
			Self::new((base & 0xffffffff) as _, limit, access_byte, flags),
			Self((base >> 32) & 0xffffffff),
		]
	}

	/// Returns the entry's base address.
	#[inline(always)]
	pub const fn get_base(&self) -> u32 {
		(((self.0 >> 16) & 0xffffff) | ((self.0 >> 32) & 0xff000000)) as _
	}

	/// Sets the entry's base address.
	#[inline(always)]
	pub const fn set_base(&mut self, base: u32) {
		self.0 &= !(0xffffff << 16);
		self.0 &= !(0xff << 56);

		self.0 |= (base as u64 & 0xffffff) << 16;
		self.0 |= ((base as u64 >> 24) & 0xff) << 56;
	}

	/// Returns the entry's limit.
	#[inline(always)]
	pub const fn get_limit(&self) -> u32 {
		((self.0 & 0xffff) | (((self.0 >> 48) & 0xf) << 16)) as _
	}

	/// Sets the entry's limit.
	///
	/// If the given limit is more than `pow(2, 20) - 1`, the value is truncated.
	#[inline(always)]
	pub const fn set_limit(&mut self, limit: u32) {
		self.0 &= !0xffff;
		self.0 &= !(0xf << 48);

		self.0 |= limit as u64 & 0xffff;
		self.0 |= ((limit as u64 >> 16) & 0xf) << 48;
	}

	/// Returns the value of the access byte.
	#[inline(always)]
	pub const fn get_access_byte(&self) -> u8 {
		((self.0 >> 40) & 0xff) as _
	}

	/// Sets the value of the access byte.
	#[inline(always)]
	pub const fn set_access_byte(&mut self, byte: u8) {
		self.0 &= !(0xff << 40);
		self.0 |= (byte as u64) << 40;
	}

	/// Returns the flags.
	#[inline(always)]
	pub const fn get_flags(&self) -> u8 {
		((self.0 >> 52) & 0x0f) as _
	}

	/// Sets the flags.
	#[inline(always)]
	pub const fn set_flags(&mut self, flags: u8) {
		self.0 &= !(0x0f << 52);
		self.0 |= ((flags as u64) & 0x0f) << 52;
	}

	/// Tells whether the entry is present.
	#[inline(always)]
	pub const fn is_present(&self) -> bool {
		(self.0 >> 47) & 1 != 0
	}

	/// Sets the entry present or not.
	#[inline(always)]
	pub const fn set_present(&mut self, present: bool) {
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
	/// An invalid offset, either not a multiple of `8` or out of bounds of the GDT, shall result
	/// in an undefined behaviour.
	///
	/// Concurrent calls are undefined.
	pub unsafe fn update_gdt(self, off: usize) {
		per_cpu()
			.gdt
			.0
			.get()
			.byte_add(off)
			.cast::<Entry>()
			.write_volatile(self);
	}
}

impl fmt::Debug for Entry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.debug_struct("Entry")
			.field("base", &self.get_base())
			.field("limit", &self.get_limit())
			.field("access_byte", &self.get_access_byte())
			.field("flags", &self.get_flags())
			.field("present", &self.is_present())
			.finish()
	}
}

/// Per-core GDT entries list.
pub struct Gdt(UnsafeCell<[Entry; 11]>);

impl Default for Gdt {
	fn default() -> Self {
		let gdt = [
			// First entry, empty
			Entry(0),
			// Kernel code segment
			#[cfg(target_arch = "x86")]
			Entry::new(0, !0, 0b10011010, 0b1100),
			#[cfg(target_arch = "x86_64")]
			Entry::new(0, !0, 0b10011010, 0b1010),
			// Kernel data segment
			Entry::new(0, !0, 0b10010010, 0b1100),
			// User code segment (32 bits)
			Entry::new(0, !0, 0b11111010, 0b1100),
			// User data segment (32 bits)
			Entry::new(0, !0, 0b11110010, 0b1100),
			// User code segment (64 bits), unused by 32 bit kernel
			Entry::new(0, !0, 0b11111010, 0b1010),
			// TSS
			Entry(0),
			Entry(0),
			// TLS entries
			Entry(0),
			Entry(0),
			Entry(0),
		];
		Gdt(UnsafeCell::new(gdt))
	}
}

/// A GDT descriptor.
#[repr(C, packed)]
struct GdtDesc {
	/// The size of the GDT in bytes, minus `1`.
	size: u16,
	/// The address to the GDT.
	addr: VirtAddr,
}

/// Flushes the current CPU's GDT.
#[inline(always)]
pub fn flush() {
	let hdr = GdtDesc {
		size: (size_of::<Gdt>() - 1) as _,
		addr: VirtAddr::from(per_cpu().gdt.0.get()),
	};
	unsafe {
		asm!("lgdt [{}]", in(reg) &hdr);
	}
}
