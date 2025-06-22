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

//! The Advanced Programmable Interrupt Controller (APIC) is the updated Intel standard for the
//! older PIC. It is necessary for the SMP support.

use super::{IA32_APIC_BASE_MSR, cpuid, rdmsr, wrmsr};
use crate::memory::PhysAddr;

/// APIC register: Spurious Interrupt Vector Register
const SPURIOUS_INTERRUPT_VECTOR_REGISTER: usize = 0xf0;

/// Tells whether the APIC is present or not.
#[inline]
pub fn is_present() -> bool {
	let edx = cpuid(1, 0, 0, 0).3;
	edx & (1 << 9) != 0
}

/// Returns the local APIC ID.
///
/// The returned value is valid only if [`is_present`] returns `true`.
#[inline]
pub fn lapic_id() -> u8 {
	let ebx = cpuid(1, 0, 0, 0).1;
	(ebx >> 24) as u8
}

/// Returns the physical base address of local APIC registers.
#[inline]
pub fn get_base_addr() -> usize {
	let val = rdmsr(IA32_APIC_BASE_MSR);
	(val & 0xffffff000) as _
}

/// Sets the physical base address of local APIC registers.
#[inline]
pub fn set_base_addr(addr: usize) {
	#[allow(overflowing_literals)]
	let val = (addr & 0xffffff000usize) | 0x800;
	wrmsr(IA32_APIC_BASE_MSR, val as _);
}

/// Reads a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present, `base_addr` is valid, and `reg` is valid.
#[inline]
pub unsafe fn read_reg(base_addr: *mut u32, reg: usize) -> u32 {
	base_addr.add(reg).read_volatile()
}

/// Writes a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present, `base_addr` is valid, and `reg` is valid.
#[inline]
pub unsafe fn write_reg(base_addr: *mut u32, reg: usize, value: u32) {
	base_addr.add(reg).write_volatile(value);
}

/// Initializes the local APIC.
pub fn init() {
	// Get base address
	let base_addr = get_base_addr();
	// Enable APIC
	set_base_addr(base_addr);
	// Setup spurious interrupt
	let base_addr = PhysAddr(base_addr).kernel_to_virtual().unwrap().as_ptr();
	unsafe {
		let val = read_reg(base_addr, SPURIOUS_INTERRUPT_VECTOR_REGISTER);
		write_reg(base_addr, SPURIOUS_INTERRUPT_VECTOR_REGISTER, val | 0x1ff);
	}
}
