//! Virtio block device implementation

use crate::{VirtioDeviceOps, le16, le32, le64};

// Feature bits

pub const VIRTIO_BLK_F_SIZE_MAX: u32 = 1 << 1;
pub const VIRTIO_BLK_F_SEG_MAX: u32 = 1 << 2;
pub const VIRTIO_BLK_F_GEOMETRY: u32 = 1 << 4;
pub const VIRTIO_BLK_F_RO: u32 = 1 << 5;
pub const VIRTIO_BLK_F_BLK_SIZE: u32 = 1 << 6;
pub const VIRTIO_BLK_F_FLUSH: u32 = 1 << 9;
pub const VIRTIO_BLK_F_TOPOLOGY: u32 = 1 << 10;
pub const VIRTIO_BLK_F_CONFIG_WCE: u32 = 1 << 11;
pub const VIRTIO_BLK_F_MQ: u32 = 1 << 12;
pub const VIRTIO_BLK_F_DISCARD: u32 = 1 << 13;
pub const VIRTIO_BLK_F_WRITE_ZEROES: u32 = 1 << 14;
pub const VIRTIO_BLK_F_LIFETIME: u32 = 1 << 15;
pub const VIRTIO_BLK_F_SECURE_ERASE: u32 = 1 << 16;
pub const VIRTIO_BLK_F_ZONED: u32 = 1 << 17;

// Legacy feature bits

pub const VIRTIO_BLK_F_BARRIER: u32 = 0;
pub const VIRTIO_BLK_F_SCSI: u32 = 7;

#[repr(C)]
struct VirtioBlkGeometry {
	cylinders: le16,
	heads: u8,
	sectors: u8,
}

#[repr(C)]
struct VirtioBlkTopology {
	physical_block_exp: u8,
	alignment_offset: u8,
	min_io_size: le16,
	opt_io_size: le32,
}

#[repr(C)]
struct VirtioBlkZonedCharacteristics {
	zone_sectors: le32,
	max_open_zones: le32,
	max_active_zones: le32,
	max_append_sectors: le32,
	write_granularity: le32,
	model: u8,
	unused2: [u8; 3],
}

#[repr(C)]
struct VirtioBlkConfig {
	capacity: le64,
	size_max: le32,
	seg_max: le32,
	geometry: VirtioBlkGeometry,
	blk_size: le32,
	topology: VirtioBlkTopology,
	writeback: u8,
	unused0: u8,
	num_queues: le16,
	max_discard_sectors: le32,
	max_discard_seg: le32,
	discard_sector_alignment: le32,
	max_write_zeroes_sectors: le32,
	max_write_zeroes_seg: le32,
	write_zeroes_may_unmap: u8,
	unused1: [u8; 3],
	max_secure_erase_sectors: le32,
	max_secure_erase_seg: le32,
	secure_erase_sector_alignment: le32,
	zoned: VirtioBlkZonedCharacteristics,
}

pub struct VirtioBlkDevOps;

impl VirtioDeviceOps for VirtioBlkDevOps {}
