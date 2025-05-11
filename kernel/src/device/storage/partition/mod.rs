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

//! A storage device can be divided into several blocks called partitions,
//! allowing for instance to install several systems on the same machine.

mod gpt;
mod mbr;

use crate::device::BlkDev;
use gpt::Gpt;
use mbr::MbrTable;
use utils::{boxed::Box, collections::vec::Vec, errno::EResult, ptr::arc::Arc};

/// A disk partition bounds.
#[derive(Debug)]
pub struct Partition {
	/// The offset to the first sector of the partition.
	pub offset: u64,
	/// The number of sectors in the partition.
	pub size: u64,
}

/// Trait representing a partition table.
pub trait Table {
	/// Reads the partition table from the given storage device `dev`.
	///
	/// If the partition table isn't present on the storage interface, the
	/// function returns `None`.
	fn read(dev: &Arc<BlkDev>) -> EResult<Option<Self>>
	where
		Self: Sized;

	/// Returns the type of the partition table.
	fn get_type(&self) -> &'static str;

	/// Reads the partitions list.
	///
	/// `dev` is the storage device on which the partitions are to be read.
	fn read_partitions(&self, dev: &Arc<BlkDev>) -> EResult<Vec<Partition>>;
}

/// Reads the list of partitions from the block device.
///
/// If no partitions table is present, the function returns `None`.
pub fn read(dev: &Arc<BlkDev>) -> EResult<Option<Box<dyn Table>>> {
	// Try GPT
	if let Some(table) = Gpt::read(dev)? {
		return Ok(Some(Box::new(table)?));
	}
	// Try MBR
	if let Some(table) = MbrTable::read(dev)? {
		return Ok(Some(Box::new(table)?));
	}
	Ok(None)
}
