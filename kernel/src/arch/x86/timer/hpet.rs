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

//! HPET (High Precision Event Timer) implementation.
//!
//! See the [HPET specification](https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/software-developers-hpet-spec-1-0a.pdf).

use crate::{
	acpi,
	acpi::{GenericAddr, TableHdr},
	arch::x86::paging::{FLAG_CACHE_DISABLE, FLAG_WRITE, FLAG_WRITE_THROUGH},
	memory::{PhysAddr, vmem::KERNEL_VMEM},
	sync::once::OnceInit,
};
use core::ptr;

/// HPET register: General Capability and ID
const REG_CAP_ID: usize = 0x0;
/// HPET register: General Configuration
const REG_GENERAL_CONFIG: usize = 0x10;
/// HPET register: Main Counter Value
const REG_MAIN_COUNTER: usize = 0xf0;

/// Base offset for timer registers
const TIMER_BASE: usize = 0x100;
/// Offset to the configuration register of a timer
const TIMER_CONFIG_OFF: usize = 0x0;
/// Offset to the comparator value register of a timer
const TIMER_COMPARATOR_OFF: usize = 0x8;

/// ACPI HPET table
#[repr(C, packed)]
pub struct AcpiHpet {
	/// The table's header
	pub header: TableHdr,
	/// Hardware revision ID, number of comparators, etc...
	pub event_timer_block_id: u32,
	/// Base address of the control registers
	pub base_address: GenericAddr,
	/// HPET sequence number
	pub hpet_number: u8,
	/// Minimum clock ticks can be set without lost interrupts while the counter is programmed to
	/// operate in periodic mode
	pub minimum_tick: u16,
	/// Page protection information
	pub page_protection: u8,
}

impl acpi::Table for AcpiHpet {
	const SIGNATURE: &'static [u8; 4] = b"HPET";
}

/// Read register at offset `off`.
unsafe fn reg_read(info: &AcpiHpet, off: usize) -> u64 {
	let addr = PhysAddr(info.base_address.address as _)
		.kernel_to_virtual()
		.unwrap();
	ptr::read_volatile(addr.as_ptr::<u64>().byte_add(off))
}

/// Write register at offset `off`.
unsafe fn reg_write(info: &AcpiHpet, off: usize, val: u64) {
	let addr = PhysAddr(info.base_address.address as _)
		.kernel_to_virtual()
		.unwrap();
	ptr::write_volatile(addr.as_ptr::<u64>().byte_add(off), val);
}

/// HPET information.
pub struct Hpet {
	/// The HPET's ACPI information
	pub acpi_info: &'static AcpiHpet,
	/// The period of a tick in nanoseconds
	pub tick_period: u32,
}

/// The HPET's information.
pub static INFO: OnceInit<Hpet> = unsafe { OnceInit::new() };

/// Initializes the HPET.
pub(crate) fn init(acpi_info: &'static AcpiHpet) {
	// Map registers
	let physaddr = PhysAddr(acpi_info.base_address.address as _);
	KERNEL_VMEM.lock().map(
		physaddr,
		physaddr.kernel_to_virtual().unwrap(),
		FLAG_WRITE | FLAG_CACHE_DISABLE | FLAG_WRITE_THROUGH,
	);
	// Read period
	let tick_period =
		unsafe { (reg_read(acpi_info, REG_CAP_ID) >> 32) as u32 }.div_ceil(1_000_000);
	let info = Hpet {
		acpi_info,
		tick_period,
	};
	unsafe {
		OnceInit::init(&INFO, info);
	}
}

/// Enables or disables the HPET.
pub fn set_enabled(enabled: bool) {
	unsafe {
		let mut val = reg_read(INFO.acpi_info, REG_GENERAL_CONFIG);
		if enabled {
			val |= 1;
		} else {
			val &= !1;
		}
		reg_write(INFO.acpi_info, REG_GENERAL_CONFIG, val);
	}
}

/// Returns the current value of the HPET main counter.
pub fn read_counter() -> u64 {
	unsafe { reg_read(INFO.acpi_info, REG_MAIN_COUNTER) }
}
