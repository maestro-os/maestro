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

//! The DSDT (Differentiated System Description Table) provides information about supported power
//! events.
//!
//! This table contains AML code which has to be parsed and executed to retrieve the required
//! information.

use super::{Table, TableHdr};
use core::mem::size_of;

/// The Differentiated System Description Table.
#[repr(C, packed)]
pub struct Dsdt {
	/// The table's header.
	pub header: TableHdr,
	/// The definition of the AML code.
	definition_block: [u8],
}

impl Dsdt {
	/// Returns a slice to the AML code.
	pub fn get_aml(&self) -> &[u8] {
		let code_len = self.header.length as usize - size_of::<TableHdr>();
		&self.definition_block[..code_len]
	}
}

impl Table for Dsdt {
	const SIGNATURE: &'static [u8; 4] = b"DSDT";
}
