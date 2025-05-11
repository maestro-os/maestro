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
//! software context switching.
//!
//! It allows to store the pointers to the stacks to use whenever an interruption happens and
//! requires switching the protection ring, and thus the stack.
//!
//! The structure has to be registered into the GDT into the TSS segment, and must be loaded using
//! instruction `ltr`.

use crate::arch::x86::gdt;
use core::{arch::asm, mem, ptr::addr_of};

/// Task State Segment.
#[repr(C)]
#[allow(missing_docs)]
#[cfg(target_arch = "x86")]
pub struct Tss {
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

/// Task State Segment.
#[repr(C, packed)]
#[allow(missing_docs)]
#[cfg(target_arch = "x86_64")]
pub struct Tss {
	pub reserved0: u32,
	pub rsp0: u64,
	pub rsp1: u64,
	pub rsp2: u64,
	pub reserved1: u64,
	pub ist1: u64,
	pub ist2: u64,
	pub ist3: u64,
	pub ist4: u64,
	pub ist5: u64,
	pub ist6: u64,
	pub ist7: u64,
	pub reserved2: u64,
	pub reserved3: u16,
	pub iopb: u16,
}

/// The Task State Segment.
#[unsafe(no_mangle)]
static mut TSS: Tss = unsafe { mem::zeroed() };

/// Initializes the TSS.
pub(crate) fn init() {
	let [gdt_entry_low, gdt_entry_high] = gdt::Entry::new64(
		addr_of!(TSS) as u64,
		size_of::<Tss>() as u32 - 1,
		0b10001001,
		0,
	);
	unsafe {
		gdt_entry_low.update_gdt(gdt::TSS_OFFSET);
		gdt_entry_high.update_gdt(gdt::TSS_OFFSET + size_of::<gdt::Entry>());
		// Sets TSS offset
		asm!(
			"mov ax, {off}",
			"ltr ax",
			off = const gdt::TSS_OFFSET
		);
	}
}

/// Sets the kernel stack pointer on the TSS.
///
/// # Safety
///
/// This function is **not** reentrant.
pub unsafe fn set_kernel_stack(kernel_stack: *mut u8) {
	#[cfg(target_arch = "x86")]
	{
		TSS.esp0 = kernel_stack as _;
		TSS.ss0 = gdt::KERNEL_DS as _;
		TSS.ss = gdt::USER_DS as _;
	}
	#[cfg(target_arch = "x86_64")]
	{
		TSS.rsp0 = kernel_stack as _;
	}
}
