//! A storage device can be divided into several blocks called partitions,
//! allowing for instance to install several systems on the same machine.

mod gpt;
mod mbr;

use super::StorageInterface;
use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use gpt::Gpt;
use mbr::MbrTable;

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
	fn read(storage: &mut dyn StorageInterface) -> Result<Option<Self>, Errno>
	where
		Self: Sized;

	/// Returns the type of the partition table.
	fn get_type(&self) -> &'static str;

	/// Reads the partitions list.
	///
	/// `storage` is the storage interface on which the partitions are to be
	/// read.
	fn get_partitions(&self, storage: &mut dyn StorageInterface) -> Result<Vec<Partition>, Errno>;
}

/// Reads the list of partitions from the given storage interface `storage`.
///
/// If no partitions table is present, the function returns `None`.
pub fn read(storage: &mut dyn StorageInterface) -> Result<Option<Box<dyn Table>>, Errno> {
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
