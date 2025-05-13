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

use crate::{acpi::rsdt::Rsdt, memory};
use core::{
	hint::{likely, unlikely},
	mem::{align_of, size_of},
	ptr, slice,
	sync::{atomic, atomic::AtomicBool},
};
use dsdt::Dsdt;
use fadt::Fadt;
use madt::Madt;

mod aml;
mod dsdt;
mod fadt;
mod madt;
mod rsdt;

// TODO use xsdt

/// The signature of the RSDP.
const RSDP_SIGNATURE: &[u8] = b"RSD PTR ";

/// Checks the checksum for `obj`.
///
/// `len` is the size of the object in bytes.
unsafe fn check_checksum<T>(obj: &T, len: usize) -> bool {
	let slice = slice::from_raw_parts(obj as *const _ as *const u8, len);
	let checksum = slice.iter().fold(0u8, |a, b| a.wrapping_add(*b));
	likely(checksum == 0)
}

/// The Root System Description Pointer (`RSDP`) is a structure storing a pointer
/// to the other structures used by ACPI.
#[repr(C)]
#[derive(Debug)]
struct Rsdp {
	/// The signature of the structure.
	signature: [u8; 8],
	/// The checksum to check against all the structure's bytes.
	checksum: u8,
	/// An OEM-supplied string that identifies the OEM.
	oemid: [u8; 6],
	/// The revision number of the structure.
	revision: u8,
	/// The address to the `RSDT`.
	rsdt_address: u32,
}

impl Rsdp {
	/// Checks that the table is valid.
	#[inline]
	pub fn check(&self) -> bool {
		if self.signature != RSDP_SIGNATURE {
			return false;
		}
		let checksum_valid = unsafe { check_checksum(self, size_of::<Self>()) };
		if !checksum_valid {
			return false;
		}
		// Check RSDT pointer
		if self.rsdt_address == 0 || self.rsdt_address as usize % align_of::<Rsdt>() != 0 {
			return false;
		}
		true
	}

	/// Returns the [`Rsdt`].
	///
	/// # Safety
	///
	/// This function is safe only if [`check`] returns `true`.
	pub unsafe fn get_rsdt(&self) -> &Rsdt {
		&*ptr::with_exposed_provenance(self.rsdt_address as _)
	}
}

/// RSDP version 2.0.
///
/// Contains the fields from [`Rsdp`], plus some extra fields.
#[repr(C)]
#[derive(Debug)]
struct Rsdp2 {
	/// The version 1.0 on structure.
	rsdp: Rsdp,

	/// The length of the structure.
	length: u32,
	/// The address to the XSDT.
	xsdt_address: u64,
	/// The checksum to check against all the structure's bytes.
	extended_checksum: u8,
	/// Reserved bytes that must not be written.
	reserved: [u8; 3],
}

/// An ACPI table header.
#[repr(C)]
#[derive(Debug)]
pub struct TableHdr {
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

impl TableHdr {
	/// Checks that the table is valid.
	pub fn check<T: Table + ?Sized>(&self) -> bool {
		if unlikely(self.signature != *T::SIGNATURE) {
			return false;
		}
		let length = self.length as usize;
		if unlikely(length < size_of::<Self>()) {
			return false;
		}
		unsafe { check_checksum(self, length) }
	}
}

/// Trait representing an ACPI table.
pub trait Table {
	/// The expected signature for the structure.
	const SIGNATURE: &'static [u8; 4];

	/// Returns a reference to the table's header.
	fn hdr(&self) -> &TableHdr {
		unsafe { &*(self as *const _ as *const TableHdr) }
	}
}

/// Finds the [`Rsdp`] and returns a reference to it.
unsafe fn find_rsdp() -> Option<&'static Rsdp> {
	let begin = (memory::PROCESS_END + 0xe0000).as_ptr();
	let end = (memory::PROCESS_END + 0xfffff).as_ptr();
	let mut ptr = begin;
	while ptr < end {
		let signature_slice = slice::from_raw_parts::<u8>(ptr, RSDP_SIGNATURE.len());
		if signature_slice == RSDP_SIGNATURE {
			return Some(&*(ptr as *const Rsdp));
		}
		ptr = ptr.add(16);
	}
	None
}

/// Boolean value telling whether the century register of the CMOS exist.
static CENTURY_REGISTER: AtomicBool = AtomicBool::new(false);

/// Tells whether the century register of the CMOS is present.
pub fn is_century_register_present() -> bool {
	CENTURY_REGISTER.load(atomic::Ordering::Relaxed)
}

/// Initializes ACPI.
///
/// This function must be called only once, at boot.
pub(crate) fn init() {
	let rsdp = unsafe { find_rsdp() };
	let Some(rsdp) = rsdp else {
		return;
	};
	if unlikely(!rsdp.check()) {
		panic!("ACPI: invalid RSDP checksum");
	}
	// Safe because `check` returned `true`
	let rsdt = unsafe { rsdp.get_rsdt() };
	// Read MADT
	if let Some(madt) = rsdt.get_table::<Madt>() {
		// Register CPU cores
		for e in madt.entries() {
			if e.entry_type == 0 {
				// TODO Register a new CPU
			}
		}
	}
	// Read FADT
	let fadt = rsdt.get_table::<Fadt>();
	if let Some(fadt) = fadt {
		CENTURY_REGISTER.store(fadt.century != 0, atomic::Ordering::Relaxed);
	}
	// Get the DSDT
	let dsdt = rsdt
		.get_table_unsized::<Dsdt>()
		.or_else(|| fadt.and_then(Fadt::get_dsdt));
	if let Some(dsdt) = dsdt {
		// Parse AML code
		let aml = dsdt.get_aml();
		let _ast = aml::parse(aml);
		// TODO
	}
}
