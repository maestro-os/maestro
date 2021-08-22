//! This modules handles ACPI's Multiple APIC Description Table (MADT).

use super::ACPITable;
use super::ACPITableHeader;

/// The offset of the entries in the MADT.
const ENTRIES_OFF: usize = 0x2c;

/// Indicates that the system also has a PC-AT-compatible dual-8259 setup (which must be disabled
/// when enabling ACPI APIC).
const PCAT_COMPAT: u32 = 0b1;

/// The Multiple APIC Description Table.
#[repr(C)]
pub struct Madt {
	/// The table's header.
	pub header: ACPITableHeader,

	/// The physical address at which each process can access its local interrupt controller.
	local_apic_addr: u32,
	/// APIC flags.
	flags: u32,
}

impl Madt {
	/// Executes the given closure for each entry in the MADT.
	pub fn foreach_entry<F: Fn(&EntryHeader)>(&self, f: F) {
		let entries_len = self.header.get_length() as usize - ENTRIES_OFF;

		let mut i = 0;
		while i < entries_len {
			let entry = unsafe {
				&*((self as *const _ as usize + ENTRIES_OFF + i) as *const EntryHeader)
			};

			f(entry);

			i += entry.get_length() as usize;
		}
	}
}

impl ACPITable for Madt {
	fn get_expected_signature() -> [u8; 4] {
		[b'A', b'P', b'I', b'C']
	}
}

/// Represents an MADT entry header.
#[repr(C)]
pub struct EntryHeader {
	/// The entry type.
	entry_type: u8,
	/// The entry length.
	length: u8,
}

impl EntryHeader {
	/// Returns the type of the entry.
	pub fn get_type(&self) -> u8 {
		self.entry_type
	}

	/// Returns the length of the entry.
	pub fn get_length(&self) -> u8 {
		self.length
	}
}
