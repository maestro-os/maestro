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
use core::{mem::size_of, ptr, ptr::Pointee};

/// Either the Root System Description Table (RSDT) or the eXtended System Description Pointer
/// (XSDP).
#[repr(C)]
#[derive(Debug)]
pub struct Sdt<const EXTENDED: bool> {
	/// The table's header
	pub header: TableHdr,
}

impl<const EXTENDED: bool> Sdt<EXTENDED> {
	/// Iterates over every ACPI tables.
	pub fn tables(&self) -> impl Iterator<Item = &TableHdr> {
		let entries_len = self.header.length as usize - size_of::<Self>();
		let entry_len = if EXTENDED {
			size_of::<u64>()
		} else {
			size_of::<u32>()
		};
		let entries_count = entries_len / entry_len;
		let entries_start = unsafe { (self as *const Self).add(1) };
		(0..entries_count).map(move |i| {
			if EXTENDED {
				unsafe {
					let entries_start = entries_start as *const u64;
					&*ptr::with_exposed_provenance(*entries_start.add(i) as usize)
				}
			} else {
				unsafe {
					let entries_start = entries_start as *const u32;
					&*ptr::with_exposed_provenance(*entries_start.add(i) as usize)
				}
			}
		})
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

impl Table for Sdt<false> {
	const SIGNATURE: &'static [u8; 4] = b"RSDT";
}

impl Table for Sdt<true> {
	const SIGNATURE: &'static [u8; 4] = b"XSDT";
}
