//! A storage device can be divided into several blocks called partitions, allowing for instance to
//! install several systems on the same machine.

mod gpt;
mod mbr;

use crate::errno::Errno;
use crate::util::container::vec::Vec;
use mbr::MBRTable;
use super::StorageInterface;

/// Structure representing a disk partition.
pub struct Partition {
	/// The offset to the first sector of the partition.
	offset: u64,
	/// The number of sectors in the partition.
	size: u64,
}

impl Partition {
	/// Creates a new instance with the given partition offset `offset` and size `size`.
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
	/// Returns the type of the partition table.
	fn get_type(&self) -> &'static str;

	/// Tells whether the parititon table is valid.
	fn is_valid(&self) -> bool;

	/// Reads the partitions list.
	fn get_partitions(&self) -> Result<Vec<Partition>, Errno>;
}

/// Reads the list of partitions from the given storage interface `storage`.
pub fn read(storage: &mut dyn StorageInterface) -> Result<Vec<Partition>, Errno> {
	// TODO Move reading MBR into the MBR module
	if storage.get_block_size() != 512 {
		return Ok(Vec::new());
	}

	let mut first_sector: [u8; 512] = [0; 512];
	storage.read(&mut first_sector, 0, 1)?;

	// Valid because taking the pointer to the buffer on the stack which has the same size as
	// the structure
	let mbr_table = unsafe {
		&*(first_sector.as_ptr() as *const MBRTable)
	};
	if mbr_table.is_valid() {
		return mbr_table.get_partitions();
	}

	// TODO Try to detect GPT

	Ok(Vec::new())
}
