//! This module handles ACPI's Root System Description Table (RSDT).

use core::mem::size_of;
use crate::memory;
use super::ACPITable;
use super::ACPITableHeader;

/// The Root System Description Table.
#[repr(C)]
pub struct Rsdt {
	/// The table's header.
	pub header: ACPITableHeader,
}

// TODO XSDT

impl Rsdt {
	/// Iterates over every ACPI tables.
	pub fn foreach_table<F: FnMut(*const ACPITableHeader)>(&self, mut f: F) {
		let entries_len = self.header.get_length() as usize - size_of::<Rsdt>();
		let entries_count = entries_len / 4;
		let entries_ptr = (self as *const _ as usize + size_of::<Rsdt>()) as *const u32;

		for i in 0..entries_count {
			let header_ptr = unsafe {
				*entries_ptr.add(i) as *const ACPITableHeader
			};

			f(header_ptr);
		}
	}

	// TODO rm?
	/// Returns a reference to the ACPI table with type T.
	pub fn get_table<T: ACPITable>(&self) -> Option<&'static T> {
		let entries_len = self.header.get_length() as usize - size_of::<Rsdt>();
		let entries_count = entries_len / size_of::<u32>();
		let entries_ptr = (self as *const _ as usize + size_of::<Rsdt>()) as *const u32;

		for i in 0..entries_count {
			let header_ptr = unsafe {
				(memory::PROCESS_END as usize + *entries_ptr.add(i) as usize)
					as *const ACPITableHeader
			};
			let header = unsafe {
				&*header_ptr
			};

			if *header.get_signature() == T::get_expected_signature() {
				let table_ptr = header_ptr as *const T;
				let table = unsafe {
					&*table_ptr
				};

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
}
