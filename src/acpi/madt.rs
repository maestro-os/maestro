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

//! This modules handles ACPI's Multiple APIC Description Table (MADT).

use super::{ACPITable, ACPITableHeader};

/// The offset of the entries in the MADT.
const ENTRIES_OFF: usize = 0x2c;

/// Indicates that the system also has a PC-AT-compatible dual-8259 setup (which
/// must be disabled when enabling ACPI APIC).
const PCAT_COMPAT: u32 = 0b1;

/// The Multiple APIC Description Table.
#[repr(C)]
#[derive(Debug)]
pub struct Madt {
	/// The table's header.
	pub header: ACPITableHeader,

	/// The physical address at which each process can access its local
	/// interrupt controller.
	local_apic_addr: u32,
	/// APIC flags.
	flags: u32,
}

impl Madt {
	/// Returns an iterator over each entry of the MADT.
	pub fn entries(&self) -> EntriesIterator {
		EntriesIterator {
			madt: self,
			cursor: 0,
		}
	}
}

impl ACPITable for Madt {
	const SIGNATURE: &'static [u8; 4] = b"APIC";
}

/// Represents an MADT entry header.
#[repr(C)]
#[derive(Debug)]
pub struct EntryHeader {
	/// The entry type.
	pub entry_type: u8,
	/// The entry length.
	pub length: u8,
}

/// Iterator over MADT entries.
pub struct EntriesIterator<'m> {
	madt: &'m Madt,
	/// Cursor.
	cursor: usize,
}

impl<'m> Iterator for EntriesIterator<'m> {
	type Item = &'m EntryHeader;

	fn next(&mut self) -> Option<Self::Item> {
		let entries_len = self.madt.header.length as usize - ENTRIES_OFF;
		if self.cursor < entries_len {
			let ptr =
				(self as *const _ as usize + ENTRIES_OFF + self.cursor) as *const EntryHeader;
			let entry = unsafe { &*ptr };
			self.cursor += entry.length as usize;
			Some(entry)
		} else {
			None
		}
	}
}
