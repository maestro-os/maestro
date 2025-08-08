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
	acpi,
	acpi::madt::{IOAPIC, Madt},
	arch::x86::paging::{FLAG_CACHE_DISABLE, FLAG_GLOBAL, FLAG_WRITE, FLAG_WRITE_THROUGH},
	memory::{PhysAddr, VirtAddr, buddy, buddy::ZONE_KERNEL, vmem::KERNEL_VMEM},
};
use core::{hint, hint::likely, ptr::null_mut};

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
fn get_base_addr() -> usize {
	let val = rdmsr(IA32_APIC_BASE_MSR);
	(val & 0xffffff000) as _
}

/// Sets the physical base address of local APIC registers.
#[inline]
fn set_base_addr(addr: usize) {
	#[allow(overflowing_literals)]
	let val = (addr & 0xfffff0000usize) | 0x800;
	wrmsr(IA32_APIC_BASE_MSR, val as _);
}

/// The pointer to the Local APIC's registers, this initialized in [`init`]
static mut BASE_ADDR: *mut u32 = null_mut();

/// Reads a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present and `reg` is valid.
#[inline]
pub unsafe fn read_reg(reg: usize) -> u32 {
	BASE_ADDR.byte_add(reg).read_volatile()
}

/// Writes a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present and `reg` is valid.
#[inline]
pub unsafe fn write_reg(reg: usize, value: u32) {
	BASE_ADDR.byte_add(reg).write_volatile(value);
}

/// Waits for the delivery of an Inter-Processor Interrupt.
///
/// # Safety
///
/// The caller must ensure the APIC is present.
pub unsafe fn wait_delivery() {
	while likely(read_reg(0x300) & (1 << 12) != 0) {
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
pub(crate) fn init() {
	// Get base address, if not done yet
	let mut base_addr = VirtAddr::from(unsafe { BASE_ADDR });
	if base_addr.is_null() {
		base_addr = PhysAddr(get_base_addr())
			.kernel_to_virtual()
			.unwrap_or_else(|| {
				// The address is too high for kernelspace. Allocate a page to move registers onto
				// it
				let phys_addr =
					buddy::alloc(0, ZONE_KERNEL).expect("not enough memory for APIC registers");
				phys_addr.kernel_to_virtual().unwrap()
			});
		unsafe {
			BASE_ADDR = base_addr.as_ptr();
		}
	}
	let base_addr_phys = base_addr.kernel_to_physical().unwrap();
	// Enable APIC
	set_base_addr(base_addr_phys.0);
	// Map registers
	KERNEL_VMEM.lock().map(
		base_addr_phys,
		base_addr,
		FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH | FLAG_WRITE | FLAG_GLOBAL,
	);
	// Setup spurious interrupt
	unsafe {
		let val = read_reg(REG_SPURIOUS_INTERRUPT_VECTOR);
		write_reg(REG_SPURIOUS_INTERRUPT_VECTOR, val | 0x1ff);
	}
}

/// Configures an I/O APIC to redirect `gsi` (Global System Interrupt) to the CPU with the given
/// local APIC ID `lapic`, at the interrupt vector `int`.
///
/// If no I/O APIC is available for `gsi`, the function does nothing and returns `false`. On
/// success, it returns `true`.
pub fn redirect_int(gsi: u32, lapic: u8, int: u8) -> bool {
	// Find the associated I/O APIC
	let ioapic = acpi::get_table::<Madt>().and_then(|madt| {
		madt.entries()
			.filter(|e| e.entry_type == 1)
			.map(|e| unsafe { e.body::<IOAPIC>() })
			.find(|ioapic| {
				let base_addr = PhysAddr(ioapic.ioapic_address as _);
				let max_entries = unsafe { ioapic_redirect_count(base_addr) } as u32;
				(ioapic.gsi..ioapic.gsi + max_entries).contains(&gsi)
			})
	});
	let Some(ioapic) = ioapic else {
		return false;
	};
	// Configure redirection
	let base_addr = PhysAddr(ioapic.ioapic_address as _);
	let i = (gsi - ioapic.gsi) as u8;
	// TODO flags
	let val = (int as u64) | ((lapic as u64) << 56);
	unsafe {
		ioapic_write(base_addr, IO_APIC_REDIRECTIONS_OFF + i * 2, val as u32);
		ioapic_write(base_addr, IO_APIC_REDIRECTIONS_OFF + i * 2 + 1, val as u32);
	}
	true
}

/// Sends an end of interrupt message to the APIC.
#[inline]
pub fn end_of_interrupt() {
	unsafe {
		write_reg(REG_EOI, 0);
	}
}
