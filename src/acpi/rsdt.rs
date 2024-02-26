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

//! This module handles ACPI's Root System Description Table (RSDT).

use super::{ACPITable, ACPITableHeader};
use core::{mem::size_of, slice};

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
	const SIGNATURE: &'static [u8; 4] = b"RSDT";
}
