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
	acpi::{madt, madt::Madt},
	memory::{PhysAddr, mmio::Mmio},
	sync::once::OnceInit,
};
use core::{hint, hint::likely, num::NonZeroUsize};
use utils::{
	collections::vec::Vec,
	errno::{AllocResult, CollectResult},
};

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

/// An I/O APIC
struct IoApic {
	/// I/O APIC's registers map
	mmio: Mmio,
	/// First Global System Interrupt handled by the I/O APIC
	gsi: u32,
}

/// Local APIC's registers map, this initialized in [`init`]
static LAPIC_MMIO: OnceInit<Mmio> = unsafe { OnceInit::new() };
/// The list of I/O APIC on the system, this initialized in [`enumerate_ioapic`]
static IO_APIC: OnceInit<Vec<IoApic>> = unsafe { OnceInit::new() };

/// Reads a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present and `reg` is valid.
#[inline]
pub unsafe fn read_reg(reg: usize) -> u32 {
	LAPIC_MMIO.as_ptr::<u32>().byte_add(reg).read_volatile()
}

/// Writes a register of the local APIC.
///
/// # Safety
///
/// The caller must ensure the APIC is present and `reg` is valid.
#[inline]
pub unsafe fn write_reg(reg: usize, value: u32) {
	LAPIC_MMIO
		.as_ptr::<u32>()
		.byte_add(reg)
		.write_volatile(value);
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
pub unsafe fn ioapic_read(base_addr: *mut u32, reg: u8) -> u32 {
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
pub unsafe fn ioapic_write(base_addr: *mut u32, reg: u8, value: u32) {
	base_addr.write_volatile(reg as _);
	base_addr.add(4).write_volatile(value);
}

/// Returns the number of redirection entries of an I/O APIC.
///
/// # Safety
///
/// The caller must ensure `base_addr` points to the registers of a valid I/O APIC.
#[inline]
pub unsafe fn ioapic_redirect_count(base_addr: *mut u32) -> u8 {
	let val = ioapic_read(base_addr, 0x1);
	let count = (val >> 16) as u8;
	count.min(24)
}

/// Initializes the local APIC.
///
/// `first` tells whether we are on the first CPU to boot.
pub(crate) fn init(first: bool) -> AllocResult<()> {
	// Map registers
	let phys_addr = get_base_addr();
	if first {
		let mmio = Mmio::new(PhysAddr(phys_addr), NonZeroUsize::new(1).unwrap(), false)?;
		unsafe {
			OnceInit::init(&LAPIC_MMIO, mmio);
		}
	}
	// Enable APIC
	set_base_addr(phys_addr);
	// Setup spurious interrupt
	unsafe {
		let val = read_reg(REG_SPURIOUS_INTERRUPT_VECTOR);
		write_reg(REG_SPURIOUS_INTERRUPT_VECTOR, val | 0x1ff);
	}
	Ok(())
}

/// Enumerates I/O APIC(s).
///
/// This function must be called only once, at boot.
pub(crate) fn enumerate_ioapic() -> AllocResult<()> {
	let Some(madt) = acpi::get_table::<Madt>() else {
		return Ok(());
	};
	let ioapics = madt
		.entries()
		.filter(|e| e.entry_type == 1)
		.map(|e| {
			let ioapic = unsafe { e.body::<madt::IOAPIC>() };
			let phys_addr = PhysAddr(ioapic.ioapic_address as _);
			let mmio = Mmio::new(phys_addr, NonZeroUsize::new(1).unwrap(), false)?;
			Ok(IoApic {
				mmio,
				gsi: ioapic.gsi,
			})
		})
		.collect::<AllocResult<CollectResult<_>>>()?
		.0?;
	unsafe {
		OnceInit::init(&IO_APIC, ioapics);
	}
	// TODO: this is necessary to use the PIT
	/*// Remap legacy interrupts
	let lapic_id = lapic_id();
	madt.entries()
		.filter(|e| e.entry_type == 2)
		.map(|e| unsafe { e.body::<InterruptSourceOverride>() })
		.for_each(|e| {
			redirect_int(e.gsi, lapic_id, 0x20 + e.irq_source);
		});*/
	Ok(())
}

/// Configures an I/O APIC to redirect `gsi` (Global System Interrupt) to the CPU with the given
/// local APIC ID `lapic`, at the interrupt vector `int`.
///
/// If no I/O APIC is available for `gsi`, the function does nothing and returns `false`. On
/// success, it returns `true`.
pub fn redirect_int(gsi: u32, lapic: u8, int: u8) -> bool {
	// Find the associated I/O APIC
	let ioapic = IO_APIC.iter().find(|ioapic| {
		let max_entries = unsafe { ioapic_redirect_count(ioapic.mmio.as_ptr()) } as u32;
		(ioapic.gsi..ioapic.gsi + max_entries).contains(&gsi)
	});
	let Some(ioapic) = ioapic else {
		return false;
	};
	// Configure redirection
	let i = (gsi - ioapic.gsi) as u8;
	// TODO flags
	let val = (int as u64) | ((lapic as u64) << 56);
	unsafe {
		ioapic_write(
			ioapic.mmio.as_ptr(),
			IO_APIC_REDIRECTIONS_OFF + i * 2,
			val as u32,
		);
		ioapic_write(
			ioapic.mmio.as_ptr(),
			IO_APIC_REDIRECTIONS_OFF + i * 2 + 1,
			val as u32,
		);
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
