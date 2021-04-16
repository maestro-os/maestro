/// This module implements ACPI related features.

use core::ffi::c_void;
use core::intrinsics::wrapping_add;
use core::mem::size_of_val;
use crate::memory;
use crate::time;
use crate::util;

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

/// TODO doc
#[repr(C)]
struct RSDP {
	/// TODO doc
	signature: [u8; 8],
	/// TODO doc
	checksum: u8,
	/// TODO doc
	oemid: [u8; 6],
	/// TODO doc
	revision: u8,
	/// TODO doc
	rsdt_address: u32,
}

/// TODO doc
#[repr(C)]
struct RSDP2 {
	/// TODO doc
	rsdp: RSDP,

	/// TODO doc
	length: u32,
	/// TODO doc
	xsdt_address: u64,
	/// TODO doc
	extended_checksum: u8,
	/// TODO doc
	reserved: [u8; 3],
}

/// Checks the checksum of the given structure.
fn check_checksum<T>(s: &T) -> bool {
	let size = size_of_val(s);
	let mut sum: u8 = 0;
	for i in 0..size {
		let byte = unsafe { // Safe since every bytes of `s` are supposed to be readable.
			*((s as *const _ as usize + i) as *const u8)
		};
		sum = wrapping_add(sum, byte);
	}

	sum == 0
}

/// Finds the RSDP and returns a reference to it.
unsafe fn find_rsdp() -> Option<&'static mut RSDP> {
	let mut i = SCAN_BEGIN;
	while i < SCAN_END {
		if util::memcmp(i, RSDP_SIGNATURE.as_ptr() as _, RSDP_SIGNATURE.len()) == 0 {
			return Some(&mut *(i as *mut RSDP));
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

	if let Some(_rsdp) = rsdp {
		// TODO Check the structure
		// TODO Get other structures
	} else {
		if time::add_clock_source(time::cmos::CMOSClock::new(false)).is_err() {
			crate::kernel_panic!("Not enough memory to create the CMOS clock source!");
		}
	}
}
