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

//! A Block Group Descriptor is a structure stored in the Block Group Descriptor
//! Table which represents a block group, which is a subdivision of the
//! filesystem.

use super::{read, write, Superblock};
use core::mem::size_of;
use utils::{errno::EResult, io::IO};

/// Structure representing a block group descriptor to be stored into the Block
/// Group Descriptor Table (BGDT).
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
	///
	/// Arguments:
	/// - `i` the id of the group descriptor to write.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	pub fn read(i: u32, superblock: &Superblock, io: &mut dyn IO) -> EResult<Self> {
		let off = (superblock.get_bgdt_offset() * superblock.get_block_size() as u64)
			+ (i as u64 * size_of::<Self>() as u64);
		unsafe { read::<Self>(off, io) }
	}

	/// Writes the current block group descriptor.
	///
	/// Arguments:
	/// - `i` the id of the group descriptor to write.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	pub fn write(&self, i: u32, superblock: &Superblock, io: &mut dyn IO) -> EResult<()> {
		let off = (superblock.get_bgdt_offset() * superblock.get_block_size() as u64)
			+ (i as u64 * size_of::<Self>() as u64);
		write(self, off, io)
	}
}
