//! This module handles ACPI's Root System Description Table (RSDT).

use super::ACPITable;
use super::ACPITableHeader;
use core::mem::size_of;
use core::slice;

/// The Root System Description Table.
#[repr(C)]
#[derive(Debug)]
pub struct Rsdt {
	/// The table's header.
	pub header: ACPITableHeader,
}

// TODO XSDT

impl Rsdt {
	/// Iterates over every ACPI tables.
	pub fn tables(&self) -> impl Iterator<Item = &ACPITableHeader> {
		let entries_len = self.header.length as usize - size_of::<Rsdt>();
		let entries_count = entries_len / size_of::<u32>();
		let entries_start = (self as *const _ as usize + size_of::<Rsdt>()) as *const u32;
		unsafe {
			slice::from_raw_parts(entries_start, entries_count)
				.iter()
				.map(|p| &*(*p as *const ACPITableHeader))
		}
	}
}

impl ACPITable for Rsdt {
	fn get_expected_signature() -> &'static [u8; 4] {
		b"RSDT"
	}
}
