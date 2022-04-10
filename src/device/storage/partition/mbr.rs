//! The Master Boot Record (MBR) is a standard partitions table format used on the x86
//! architecture.
//! The partition table is located on the first sector of the boot disk, alongside with the boot
//! code.

use crate::errno::Errno;
use crate::util::container::vec::Vec;
use super::Partition;
use super::Table;

/// The signature of the MBR partition table.
const MBR_SIGNATURE: u16 = 0x55aa;

/// Structure representing a partition.
#[repr(C, packed)]
pub struct MBRPartition {
	/// Partition attributes.
	attrs: u8,
	/// CHS address of partition start.
	chs_start: [u8; 3],
	/// The type of the partition.
	parition_type: u8,
	/// CHS address of partition end.
	chs_end: [u8; 3],
	/// LBA address of partition start.
	lba_start: u32,
	/// The number of sectors in the partition.
	sectors_count: u32,
}

impl MBRPartition {
	/// Tells whether the partition is active.
	pub fn is_active(&self) -> bool {
		self.attrs & (1 << 7) != 0
	}
}

/// Structure representing the partition table.
#[repr(C, packed)]
pub struct MBRTable {
	/// The boot code.
	boot: [u8; 440],
	/// The disk signature (optional).
	disk_signature: u32,
	/// Zero.
	zero: u16,
	/// The list of partitions.
	partitions: [MBRPartition; 4],
	/// The partition table signature.
	signature: u16,
}

impl Table for MBRTable {
	fn get_type(&self) -> &'static str {
		"MBR"
	}

	fn is_valid(&self) -> bool {
		self.signature == MBR_SIGNATURE
	}

	fn read(&self) -> Result<Vec<Partition>, Errno> {
		let mut partitions = Vec::<Partition>::new();

		for mbr_partition in self.partitions.iter() {
			if mbr_partition.is_active() {
				let partition = Partition::new(mbr_partition.lba_start as _,
					mbr_partition.sectors_count as _);
				partitions.push(partition)?;
			}
		}

		Ok(partitions)
	}
}
