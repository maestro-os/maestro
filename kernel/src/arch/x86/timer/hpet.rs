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
	time::HwTimer,
};

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

/// Structure representing the HPET
pub struct Hpet;

impl HwTimer for Hpet {
	fn set_enabled(&mut self, _enable: bool) {
		todo!()
	}

	fn set_frequency(&mut self, _freq: u32) {
		todo!()
	}

	fn get_interrupt_vector(&self) -> u32 {
		todo!()
	}
}

/// Initializes the HPET.
pub(crate) fn init(_info: &AcpiHpet) -> Hpet {
	todo!()
}
