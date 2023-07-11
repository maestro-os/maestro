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
	signature: [u8; 4],
	/// The length of the structure.
	length: u32,
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
	/// Returns the name of the table.
	#[inline(always)]
	pub fn get_signature(&self) -> &[u8; 4] {
		&self.signature
	}

	/// Returns the length of the table.
	#[inline(always)]
	pub fn get_length(&self) -> usize {
		self.length as _
	}

	/// Checks that the table is valid.
	pub fn check<T: ACPITable + ?Sized>(&self) -> bool {
		if self.signature != *T::get_expected_signature() {
			return false;
		}

		let length = self.get_length();
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
	/// Returns the expected signature for the structure.
	fn get_expected_signature() -> &'static [u8; 4];

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
pub fn init() {
	// Reading ACPI data
	let data = ACPIData::read().unwrap_or_else(|_| {
		crate::kernel_panic!("Invalid ACPI data!");
	});

	if let Some(data) = data {
		if let Some(madt) = data.get_table_sized::<Madt>() {
			// Registering CPU cores
			madt.foreach_entry(|e: &madt::EntryHeader| match e.get_type() {
				0 => {
					// TODO Register a new CPU
				}

				_ => {}
			});
		}

		// Setting the century register value
		unsafe {
			// Safe because the value is only set once
			CENTURY_REGISTER = data
				.get_table_sized::<Fadt>()
				.map_or(false, |fadt| fadt.century != 0);
		}

		// Getting the DSDT
		let dsdt = data.get_table_unsized::<Dsdt>().or_else(|| {
			data.get_table_sized::<Fadt>()
				.and_then(|fadt| fadt.get_dsdt())
		});
		if let Some(dsdt) = dsdt {
			// Parsing AML code
			let aml = dsdt.get_aml();
			let _ast = aml::parse(aml);

			// TODO
		}
	}
}
