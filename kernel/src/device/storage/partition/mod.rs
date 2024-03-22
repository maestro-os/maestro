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

use super::StorageInterface;
use gpt::Gpt;
use mbr::MbrTable;
use utils::{boxed::Box, collections::vec::Vec, errno::EResult};

/// Structure representing a disk partition.
pub struct Partition {
	/// The offset to the first sector of the partition.
	offset: u64,
	/// The number of sectors in the partition.
	size: u64,
}

impl Partition {
	/// Creates a new instance with the given partition offset `offset` and size
	/// `size`.
	pub fn new(offset: u64, size: u64) -> Self {
		Self {
			offset,
			size,
		}
	}

	/// Returns the offset of the first sector of the partition.
	#[inline]
	pub fn get_offset(&self) -> u64 {
		self.offset
	}

	/// Returns the number of sectors in the partition.
	#[inline]
	pub fn get_size(&self) -> u64 {
		self.size
	}
}

/// Trait representing a partition table.
pub trait Table {
	/// Reads the partition table from the given storage interface `storage`.
	///
	/// If the partition table isn't present on the storage interface, the
	/// function returns `None`.
	fn read(storage: &mut dyn StorageInterface) -> EResult<Option<Self>>
	where
		Self: Sized;

	/// Returns the type of the partition table.
	fn get_type(&self) -> &'static str;

	/// Reads the partitions list.
	///
	/// `storage` is the storage interface on which the partitions are to be
	/// read.
	fn get_partitions(&self, storage: &mut dyn StorageInterface) -> EResult<Vec<Partition>>;
}

/// Reads the list of partitions from the given storage interface `storage`.
///
/// If no partitions table is present, the function returns `None`.
pub fn read(storage: &mut dyn StorageInterface) -> EResult<Option<Box<dyn Table>>> {
	// Try GPT
	if let Some(table) = Gpt::read(storage)? {
		return Ok(Some(Box::new(table)?));
	}
	// Try MBR
	if let Some(table) = MbrTable::read(storage)? {
		return Ok(Some(Box::new(table)?));
	}
	Ok(None)
}
