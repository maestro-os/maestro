//! This module implements ACPI related features.

use core::ffi::c_void;
use core::intrinsics::wrapping_add;
use crate::memory;
use crate::time;
use crate::util;
use fadt::Fadt;
use madt::Madt;
use rsdt::Rsdt;

mod fadt;
mod madt;
mod rsdt;

/// The beginning of the zone to scan to get the RSDP.
const SCAN_BEGIN: *const c_void = unsafe {
	memory::PROCESS_END as usize + 0xe0000
} as *const c_void;
/// The end of the zone to scan to get the RSDP.
const SCAN_END: *const c_void = unsafe {
	memory::PROCESS_END as usize + 0xfffff
} as *const c_void;

/// The signature of the RSDP structure.
const RSDP_SIGNATURE: &str = "RSD PTR ";

/// Trait representing an ACPI table.
pub trait ACPITable {
	/// Returns the expected signature for the structure.
	fn get_expected_signature() -> [u8; 4];

	/// Returns the name of the table.
	fn get_signature(&self) -> &[u8; 4];
	/// Returns the length of the table.
	fn get_length(&self) -> usize;

	/// Checks that the table is valid.
	fn check(&self) -> bool {
		let length = self.get_length();
		let mut sum: u8 = 0;

		for i in 0..length {
			let byte = unsafe { // Safe since every bytes of `s` are readable.
				*((self as *const Self as *const u8 as usize + i) as *const u8)
			};
			sum = wrapping_add(sum, byte);
		}

		sum == 0
	}
}

/// The Root System Description Pointer (RSDP) is a structure storing a pointer to the other
/// structures used by ACPI.
#[repr(C)]
struct Rsdp {
	/// The signature of the structure.
	signature: [u8; 8],
	/// The checksum to check against all the structure's bytes.
	checksum: u8,
	/// An OEM-supplied string that identifies the OEM.
	oemid: [u8; 6],
	/// The revision number of the structure.
	revision: u8,
	/// The address to the RSDT.
	rsdt_address: u32,
}

/// This structure is the version 2.0 of the RSDP. This structure contains the field from the
/// previous version, plus some extra fields.
#[repr(C)]
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

/// Finds the RSDP and returns a reference to it.
unsafe fn find_rsdp() -> Option<&'static mut Rsdp> {
	let mut i = SCAN_BEGIN;

	while i < SCAN_END {
		if util::memcmp(i, RSDP_SIGNATURE.as_ptr() as _, RSDP_SIGNATURE.len()) == 0 {
			return Some(&mut *(i as *mut Rsdp));
		}

		i = i.add(16);
	}

	None
}

/// Initializes ACPI.
pub fn init() {
	let rsdp = unsafe {
		find_rsdp()
	};

	let mut century_register = false;

	if let Some(rsdp) = rsdp {
		// TODO Check rsdp
		let rsdt = unsafe {
			&*((memory::PROCESS_END as usize + rsdp.rsdt_address as usize) as *const Rsdt)
		};
		if !rsdt.check() {
			crate::kernel_panic!("Invalid ACPI structure!");
		}

		if let Some(madt) = rsdt.get_table::<Madt>() {
			madt.foreach_entry(| e: &madt::EntryHeader | {
				match e.get_type() {
					0 => {
						// TODO Register a new CPU
					},

					_ => {},
				}
			});
		}

		century_register = {
			if let Some(fadt) = rsdt.get_table::<Fadt>() {
				fadt.century != 0
			} else {
				false
			}
		};
	}

	let cmos_clock = time::cmos::CMOSClock::new(century_register);
	if time::add_clock_source(cmos_clock).is_err() {
		crate::kernel_panic!("Not enough memory to create the CMOS clock source!");
	}
}
