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

use super::{Table, TableHdr};
use core::{mem::size_of, ptr, ptr::Pointee, slice};

/// The Root System Description Table.
#[repr(C)]
#[derive(Debug)]
pub struct Rsdt {
	/// The table's header.
	pub header: TableHdr,
}

// TODO XSDT

impl Rsdt {
	/// Iterates over every ACPI tables.
	pub fn tables(&self) -> impl Iterator<Item = &TableHdr> {
		let entries_len = self.header.length as usize - size_of::<Rsdt>();
		let entries_count = entries_len / size_of::<u32>();
		unsafe {
			let entries_start = (self as *const Self).add(1) as *const u32;
			slice::from_raw_parts(entries_start, entries_count)
				.iter()
				.map(|p| &*ptr::with_exposed_provenance(*p as usize))
		}
	}

	/// Returns a reference to the ACPI table with type `T`.
	///
	/// If the table does not exist, the function returns `None`.
	///
	/// If the table is invalid, the function panics.
	pub fn get_table<T: Table>(&self) -> Option<&T> {
		let hdr = self.tables().find(|hdr| hdr.signature == *T::SIGNATURE)?;
		if !hdr.check::<T>() {
			panic!("APCI: invalid table for signature {:?}", hdr.signature)
		}
		Some(unsafe { &*(hdr as *const _ as *const T) })
	}

	/// Returns a reference to the ACPI table with type `T`.
	///
	/// The table must be `Unsized`.
	///
	/// If the table doesn't exist, the function returns `None`.
	pub fn get_table_unsized<T: Table + ?Sized + Pointee<Metadata = usize>>(&self) -> Option<&T> {
		let hdr = self.tables().find(|hdr| hdr.signature == *T::SIGNATURE)?;
		if !hdr.check::<T>() {
			panic!("APCI: invalid table for signature {:?}", hdr.signature)
		}
		Some(unsafe {
			let ptr = ptr::from_raw_parts::<T>(hdr as *const _ as *const (), hdr.length as usize);
			&*ptr
		})
	}
}

impl Table for Rsdt {
	const SIGNATURE: &'static [u8; 4] = b"RSDT";
}
