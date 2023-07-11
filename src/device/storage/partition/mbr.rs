//! The Master Boot Record (MBR) is a standard partitions table format used on
//! the x86 architecture.
//!
//! The partition table is located on the first sector of the boot disk,
//! alongside with the boot code.

use super::Partition;
use super::Table;
use crate::device::storage::StorageInterface;
use crate::errno::Errno;
use crate::util::container::vec::Vec;

/// The signature of the MBR partition table.
const MBR_SIGNATURE: u16 = 0xaa55;

/// Structure representing a partition.
#[derive(Clone)]
#[repr(C, packed)]
struct MbrPartition {
	/// Partition attributes.
	attrs: u8,
	/// CHS address of partition start.
	chs_start: [u8; 3],
	/// The type of the partition.
	partition_type: u8,
	/// CHS address of partition end.
	chs_end: [u8; 3],
	/// LBA address of partition start.
	lba_start: u32,
	/// The number of sectors in the partition.
	sectors_count: u32,
}

/// Structure representing the partition table.
#[repr(C, packed)]
pub struct MbrTable {
	/// The boot code.
	boot: [u8; 440],
	/// The disk signature (optional).
	disk_signature: u32,
	/// Zero.
	zero: u16,
	/// The list of partitions.
	partitions: [MbrPartition; 4],
	/// The partition table signature.
	signature: u16,
}

impl Clone for MbrTable {
	fn clone(&self) -> Self {
		Self {
			boot: self.boot,
			disk_signature: self.disk_signature,
			zero: self.zero,
			partitions: self.partitions.clone(),
			signature: self.signature,
		}
	}
}

impl Table for MbrTable {
	fn read(storage: &mut dyn StorageInterface) -> Result<Option<Self>, Errno> {
		let mut first_sector: [u8; 512] = [0; 512];

		if first_sector.len() as u64 > storage.get_size() {
			return Ok(None);
		}
		storage.read_bytes(&mut first_sector, 0)?;

		// Valid because taking the pointer to the buffer on the stack which has the
		// same size as the structure
		let mbr_table = unsafe { &*(first_sector.as_ptr() as *const MbrTable) };
		if mbr_table.signature != MBR_SIGNATURE {
			return Ok(None);
		}

		Ok(Some(mbr_table.clone()))
	}

	fn get_type(&self) -> &'static str {
		"MBR"
	}

	fn get_partitions(&self, _: &mut dyn StorageInterface) -> Result<Vec<Partition>, Errno> {
		let mut partitions = Vec::<Partition>::new();

		for mbr_partition in self.partitions.iter() {
			if mbr_partition.partition_type != 0 {
				let partition = Partition::new(
					mbr_partition.lba_start as _,
					mbr_partition.sectors_count as _,
				);
				partitions.push(partition)?;
			}
		}

		Ok(partitions)
	}
}
