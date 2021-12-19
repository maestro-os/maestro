//! A Block Group Descriptor is a structure stored in the Block Group Descriptor Table which
//! represents a block group, which is a subdivision of the filesystem.

use core::mem::size_of;
use crate::errno::Errno;
use crate::util::IO;
use super::Superblock;
use super::read;
use super::write;

/// Structure representing a block group descriptor to be stored into the Block Group Descriptor
/// Table (BGDT).
#[repr(C, packed)]
pub struct BlockGroupDescriptor {
	/// The block address of the block usage bitmap.
	pub block_usage_bitmap_addr: u32,
	/// The block address of the inode usage bitmap.
	pub inode_usage_bitmap_addr: u32,
	/// Starting block address of inode table.
	pub inode_table_start_addr: u32,
	/// Number of unallocated blocks in group.
	pub unallocated_blocks_number: u16,
	/// Number of unallocated inodes in group.
	pub unallocated_inodes_number: u16,
	/// Number of directories in group.
	pub directories_number: u16,

	/// Structure padding.
	pub _padding: [u8; 14],
}

impl BlockGroupDescriptor {
	/// Reads the `i`th block group descriptor from the given device.
	/// `i` the id of the group descriptor to write.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	pub fn read(i: u32, superblock: &Superblock, io: &mut dyn IO)
		-> Result<Self, Errno> {
		let off = (superblock.get_bgdt_offset() * superblock.get_block_size() as u64)
			+ (i as u64 * size_of::<Self>() as u64);
		unsafe {
			read::<Self>(off, io)
		}
	}

	/// Writes the current block group descriptor.
	/// `i` the id of the group descriptor to write.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	pub fn write(&self, i: u32, superblock: &Superblock, io: &mut dyn IO)
		-> Result<(), Errno> {
		let off = (superblock.get_bgdt_offset() * superblock.get_block_size() as u64)
			+ (i as u64 * size_of::<Self>() as u64);
		write(self, off, io)
	}
}
