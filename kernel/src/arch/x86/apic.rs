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

// TODO implement x2APIC

use super::{IA32_APIC_BASE_MSR, cpuid, rdmsr, wrmsr};
use crate::{
	arch::x86::paging::{FLAG_CACHE_DISABLE, FLAG_GLOBAL, FLAG_WRITE, FLAG_WRITE_THROUGH},
	memory::{PhysAddr, vmem::KERNEL_VMEM},
};
use core::{hint, hint::likely};
use utils::limits::PAGE_SIZE;

/// APIC register: Local APIC ID
pub const REG_EOI: usize = 0xb0;
/// APIC register: Spurious Interrupt Vector Register
pub const REG_SPURIOUS_INTERRUPT_VECTOR: usize = 0xf0;
/// APIC register: Error status
pub const REG_ERROR_STATUS: usize = 0x280;
/// APIC register: Interrupt Command Register (low)
pub const REG_ICR_LO: usize = 0x300;
/// APIC register: Interrupt Command Register (high)
pub const REG_ICR_HI: usize = 0x310;
/// APIC register: LVT Timer
pub const REG_LVT_TIMER: usize = 0x320;
/// APIC register: Initial Count Register
pub const REG_TIMER_INIT_COUNT: usize = 0x380;
/// APIC register: Current Count Register
pub const REG_TIMER_CURRENT_COUNT: usize = 0x390;
/// APIC register: Divide Configuration Register
pub const REG_TIMER_DIVIDE: usize = 0x3e0;

/// LVT flag: mask interrupt
pub const LVT_MASKED: u32 = 1 << 16;
/// LVT mode: Oneshot
pub const LVT_ONESHOT: u32 = 0b00 << 17;
/// LVT mode: Periodic
pub const LVT_PERIODIC: u32 = 0b01 << 17;
/// LVT mode: TSC deadline
pub const LVT_TSC_DEADLINE: u32 = 0b10 << 17;

/// I/O APIC: redirection entries registers offset
pub const IO_APIC_REDIRECTIONS_OFF: u8 = 0x10;

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
	let val = (addr & 0xfffff0000usize) | 0x800;
	wrmsr(IA32_APIC_BASE_MSR, val as _);
}

/// Reads a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present, `base_addr` is valid, and `reg` is valid.
#[inline]
pub unsafe fn read_reg(base_addr: *mut u32, reg: usize) -> u32 {
	base_addr.byte_add(reg).read_volatile()
}

/// Writes a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present, `base_addr` is valid, and `reg` is valid.
#[inline]
pub unsafe fn write_reg(base_addr: *mut u32, reg: usize, value: u32) {
	base_addr.byte_add(reg).write_volatile(value);
}

/// Waits for the delivery of an Inter-Processor Interrupt.
///
/// # Safety
///
/// The caller must ensure the APIC is present, `base_addr` is valid.
pub unsafe fn wait_delivery(base_addr: *mut u32) {
	while likely(read_reg(base_addr, 0x300) & (1 << 12) != 0) {
		hint::spin_loop();
	}
}

/// Reads a register of the I/O APIC.
///
/// # Safety
///
/// The caller must ensure `base_addr` points to the registers of a valid I/O APIC and `reg` is
/// valid.
#[inline]
pub unsafe fn ioapic_read(base_addr: PhysAddr, reg: u8) -> u32 {
	let base_addr: *mut u32 = base_addr.kernel_to_virtual().unwrap().as_ptr();
	base_addr.write_volatile(reg as _);
	base_addr.add(4).read_volatile()
}

/// Writes a register of the I/O APIC.
///
/// # Safety
///
/// The caller must ensure `base_addr` points to the registers of a valid I/O APIC and `reg` is
/// valid.
#[inline]
pub unsafe fn ioapic_write(base_addr: PhysAddr, reg: u8, value: u32) {
	let base_addr: *mut u32 = base_addr.kernel_to_virtual().unwrap().as_ptr();
	base_addr.write_volatile(reg as _);
	base_addr.add(4).write_volatile(value);
}

/// Returns the number of redirection entries of an I/O APIC.
///
/// # Safety
///
/// The caller must ensure `base_addr` points to the registers of a valid I/O APIC.
#[inline]
pub unsafe fn ioapic_redirect_count(base_addr: PhysAddr) -> u8 {
	let val = ioapic_read(base_addr, 0x1);
	let count = (val >> 16) as u8;
	count.min(24)
}

/// Initializes the local APIC.
pub fn init() {
	// Get base address
	let base_addr = get_base_addr();
	// Enable APIC
	set_base_addr(base_addr);
	// Map registers
	let phys_addr = PhysAddr(base_addr).down_align_to(PAGE_SIZE);
	KERNEL_VMEM.lock().map(
		phys_addr,
		phys_addr.kernel_to_virtual().unwrap(),
		FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH | FLAG_WRITE | FLAG_GLOBAL,
	);
	// Setup spurious interrupt
	let base_addr = PhysAddr(base_addr).kernel_to_virtual().unwrap().as_ptr();
	unsafe {
		let val = read_reg(base_addr, REG_SPURIOUS_INTERRUPT_VECTOR);
		write_reg(base_addr, REG_SPURIOUS_INTERRUPT_VECTOR, val | 0x1ff);
	}
}

/// Sends an end of interrupt message to the APIC.
pub fn end_of_interrupt() {
	// TODO cache
	let base_addr = get_base_addr();
	let base_addr = PhysAddr(base_addr).kernel_to_virtual().unwrap().as_ptr();
	unsafe {
		write_reg(base_addr, REG_EOI, 0);
	}
}
