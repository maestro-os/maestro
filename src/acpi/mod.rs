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

//! The ACPI (Advanced Configuration and Power Interface) interface provides informations about the
//! system, allowing to control components such as cooling and power.
//!
//! ACPI initialization is done through the following phases:
//! - Read the `RSDP` table in order to get a pointer to the `RSDT`, referring to every other
//!   available tables.
//! - TODO

use core::mem::size_of;
use data::ACPIData;
use dsdt::Dsdt;
use fadt::Fadt;
use madt::Madt;

mod aml;
mod data;
mod dsdt;
mod fadt;
mod madt;
mod rsdt;

/// An ACPI table header.
#[repr(C)]
#[derive(Debug)]
pub struct ACPITableHeader {
	/// The signature of the structure.
	pub signature: [u8; 4],
	/// The length of the structure.
	pub length: u32,
	/// The revision number of the structure.
	revision: u8,
	/// The checksum to check against all the structure's bytes.
	checksum: u8,
	/// An OEM-supplied string that identifies the OEM.
	oemid: [u8; 6],
	/// The manufacturer model ID.
	oem_table_id: [u8; 8],
	/// OEM revision for supplied OEM table ID.
	oemrevision: u32,
	/// Vendor ID of utility that created the table.
	creator_id: u32,
	/// Revision of utility that created the table.
	creator_revision: u32,
}

impl ACPITableHeader {
	/// Checks that the table is valid.
	pub fn check<T: ACPITable + ?Sized>(&self) -> bool {
		if self.signature != *T::SIGNATURE {
			return false;
		}

		let length = self.length as usize;
		if length < size_of::<Self>() {
			return false;
		}

		let mut sum: u8 = 0;

		for i in 0..length {
			let byte = unsafe {
				// Safe since every bytes of `self` are readable.
				*(self as *const Self as *const u8).add(i)
			};
			sum = sum.wrapping_add(byte);
		}

		sum == 0
	}
}

/// Trait representing an ACPI table.
pub trait ACPITable {
	/// The expected signature for the structure.
	const SIGNATURE: &'static [u8; 4];

	/// Returns a reference to the table's header.
	fn get_header(&self) -> &ACPITableHeader {
		unsafe { &*(self as *const _ as *const ACPITableHeader) }
	}
}

/// Boolean value telling whether the century register of the CMOS exist.
static mut CENTURY_REGISTER: bool = false;

/// Tells whether the century register of the CMOS is present.
pub fn is_century_register_present() -> bool {
	unsafe {
		// Safe because the value is only set once at boot
		CENTURY_REGISTER
	}
}

/// Initializes ACPI.
///
/// This function must be called only once, at boot.
pub(crate) fn init() {
	// Read ACPI data
	let data = ACPIData::read().unwrap_or_else(|_| {
		panic!("Invalid ACPI data!");
	});

	if let Some(data) = data {
		if let Some(madt) = data.get_table_sized::<Madt>() {
			// Register CPU cores
			for e in madt.entries() {
				if e.entry_type == 0 {
					// TODO Register a new CPU
				}
			}
		}

		// Set the century register value
		unsafe {
			// Safe because the value is only set once
			CENTURY_REGISTER = data
				.get_table_sized::<Fadt>()
				.map_or(false, |fadt| fadt.century != 0);
		}

		// Get the DSDT
		let dsdt = data
			.get_table_unsized::<Dsdt>()
			.or_else(|| data.get_table_sized::<Fadt>().and_then(Fadt::get_dsdt));
		if let Some(dsdt) = dsdt {
			// Parse AML code
			let aml = dsdt.get_aml();
			let _ast = aml::parse(aml);

			// TODO
		}
	}
}
