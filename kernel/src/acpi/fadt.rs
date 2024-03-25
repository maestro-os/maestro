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

//! This module handles ACPI's Fixed ACPI Description Table (FADT).

use super::{dsdt::Dsdt, ACPITable, ACPITableHeader};
use core::slice;

/// TODO doc
pub struct GenericAddr {
	addr_space: u8,
	bit_width: u8,
	bit_offset: u8,
	access_size: u8,
	address: u8,
}

/// The Fixed ACPI Description Table.
///
/// The documentation of every fields can be found in the ACPI documentation.
#[repr(C)]
pub struct Fadt {
	/// The table's header.
	pub header: ACPITableHeader,

	pub firmware_ctrl: u32,
	pub dsdt: u32,

	pub reserved: u8,

	pub preferred_power_management_profile: u8,
	pub sci_interrupt: u16,
	pub smi_commandport: u32,
	pub acpi_enable: u8,
	pub acpi_disable: u8,
	pub s4bios_req: u8,
	pub pstate_control: u8,
	pub pm1a_event_block: u32,
	pub pm1b_event_block: u32,
	pub pm1a_control_block: u32,
	pub pm1b_control_block: u32,
	pub pm2c_ontrolb_lock: u32,
	pub pm_timer_block: u32,
	pub gpe0_block: u32,
	pub gpe1_block: u32,
	pub pm1_event_length: u8,
	pub pm1_control_length: u8,
	pub pm2_control_length: u8,
	pub pm_timer_length: u8,
	pub gpe0_length: u8,
	pub gpe1_length: u8,
	pub gpe1_base: u8,
	pub cstate_control: u8,
	pub worst_c2_latency: u16,
	pub worst_c3_latency: u16,
	pub flush_size: u16,
	pub flush_stride: u16,
	pub duty_offset: u8,
	pub duty_width: u8,
	pub day_alarm: u8,
	pub month_alarm: u8,
	pub century: u8,

	pub boot_architecture_flags: u16,

	pub reserved2: u8,
	pub flags: u32,

	pub reset_reg: GenericAddr,

	pub reset_value: u8,
	pub reserved3: [u8; 3],

	pub x_firmware_control: u64,
	pub x_dsdt: u64,

	pub x_pm1a_event_block: GenericAddr,
	pub x_pm1b_event_block: GenericAddr,
	pub x_pm1a_control_block: GenericAddr,
	pub x_pm1b_control_block: GenericAddr,
	pub x_pm2_control_block: GenericAddr,
	pub x_pm_timer_block: GenericAddr,
	pub x_gpe0_block: GenericAddr,
	pub x_gpe1_block: GenericAddr,
}

impl Fadt {
	/// Returns a reference to the DSDT if it exists.
	pub fn get_dsdt(&self) -> Option<&Dsdt> {
		let dsdt = if self.x_dsdt != 0 {
			self.x_dsdt
		} else {
			self.dsdt as _
		} as *const u8;
		// TODO Add displacement relative to ACPI data buffer

		if !dsdt.is_null() {
			let dsdt = unsafe {
				let dsdt_slice = slice::from_raw_parts(dsdt, 1);
				&*(dsdt_slice as *const [u8] as *const [()] as *const Dsdt)
			};

			if !dsdt.get_header().check::<Dsdt>() {
				panic!("Invalid ACPI structure!");
			}

			Some(dsdt)
		} else {
			None
		}
	}
}

impl ACPITable for Fadt {
	const SIGNATURE: &'static [u8; 4] = b"FACP";
}
