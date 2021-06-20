//! This module handles ACPI's Root System Description Table (RSDT).

use core::mem::size_of;
use super::ACPITable;

/// The Root System Description Table.
#[repr(C)]
pub struct Rsdt {
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
	/// TODO doc
	oem_table_id: [u8; 8],
	/// TODO doc
	oemrevision: u32,
	/// TODO doc
	creator_id: u32,
	/// TODO doc
	creator_revision: u32,
}

// TODO XSDT

impl Rsdt {
	/// Returns a reference to the ACPI table with type T.
	pub fn get_table<T: ACPITable>(&self) -> Option<&'static T> {
		let entries_len = self.length as usize - size_of::<Rsdt>();
		let entries_count = entries_len / 4;

		for i in 0..entries_count {
			let ptr = (self as *const _ as usize + size_of::<Rsdt>()) + (i * size_of::<u32>());
			let table = unsafe {
				&*(ptr as *const T)
			};

			if *table.get_signature() == T::get_expected_signature() {
				return Some(table);
			}
		}

		None
	}
}

impl ACPITable for Rsdt {
	fn get_expected_signature() -> [u8; 4] {
		[b'R', b'S', b'D', b'T']
	}

	fn get_signature(&self) -> &[u8; 4] {
		&self.signature
	}

	fn get_length(&self) -> usize {
		self.length as _
	}
}
