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

//! Under the x86 architecture, the TSS (Task State Segment) is a structure that
//! is mostly deprecated but that must still be used in order to perform
//! software context switching because it allows to store the pointers to the
//! stacks to use whenever an interruption happens and requires switching the
//! protection ring, and thus the stack.
//!
//! The structure has to be registered into the GDT into the TSS segment, and must be loaded using
//! instruction `ltr`.

use crate::gdt;
use core::{arch::asm, mem::size_of, ptr::addr_of};

/// The TSS structure.
#[repr(C, packed)]
pub struct TSS {
	pub prev_tss: u32,
	pub esp0: u32,
	pub ss0: u32,
	pub esp1: u32,
	pub ss1: u32,
	pub esp2: u32,
	pub ss2: u32,
	pub cr3: u32,
	pub eip: u32,
	pub eflags: u32,
	pub eax: u32,
	pub ecx: u32,
	pub edx: u32,
	pub ebx: u32,
	pub esp: u32,
	pub ebp: u32,
	pub esi: u32,
	pub edi: u32,
	pub es: u32,
	pub cs: u32,
	pub ss: u32,
	pub ds: u32,
	pub fs: u32,
	pub gs: u32,
	pub ldt: u32,
	pub trap: u16,
	pub iomap_base: u16,
}

impl TSS {
	/// Creates a new zeroed instance.
	const fn new() -> Self {
		Self {
			prev_tss: 0,
			esp0: 0,
			ss0: 0,
			esp1: 0,
			ss1: 0,
			esp2: 0,
			ss2: 0,
			cr3: 0,
			eip: 0,
			eflags: 0,
			eax: 0,
			ecx: 0,
			edx: 0,
			ebx: 0,
			esp: 0,
			ebp: 0,
			esi: 0,
			edi: 0,
			es: 0,
			cs: 0,
			ss: 0,
			ds: 0,
			fs: 0,
			gs: 0,
			ldt: 0,
			trap: 0,
			iomap_base: 0,
		}
	}

	/// Initializes the TSS.
	pub fn init() {
		let limit = size_of::<Self>() as u64;
		let base = unsafe { addr_of!(TSS) as u64 };
		let flags = 0b0100000010001001_u64;
		let tss_value = (limit & 0xffff)
			| ((base & 0xffffff) << 16)
			| (flags << 40)
			| (((limit >> 16) & 0x0f) << 48)
			| (((base >> 24) & 0xff) << 56);

		let gdt_entry = gdt::Entry(tss_value);
		unsafe {
			gdt_entry.update_gdt(gdt::TSS_OFFSET);
		}
		Self::flush();
	}

	/// Updates the TSS into the GDT.
	#[inline(always)]
	pub fn flush() {
		unsafe {
			asm!(
				"mov ax, {off}",
				"ltr ax",
				off = const gdt::TSS_OFFSET
			);
		}
		gdt::flush();
	}
}

/// Wrapper for memory alignment.
#[repr(align(4096))]
pub struct TSSWrap(pub TSS);

/// The Task State Segment.
#[no_mangle]
pub static mut TSS: TSSWrap = TSSWrap(TSS::new());
