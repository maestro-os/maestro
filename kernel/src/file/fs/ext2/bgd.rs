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
use crate::device::DeviceIO;
use core::mem::size_of;
use macros::AnyRepr;
use utils::errno::EResult;

/// A block group descriptor.
#[repr(C)]
#[derive(AnyRepr, Clone)]
pub struct BlockGroupDescriptor {
	/// The block address of the block usage bitmap.
	pub bg_block_bitmap: u32,
	/// The block address of the inode usage bitmap.
	pub bg_inode_bitmap: u32,
	/// Starting block address of inode table.
	pub bg_inode_table: u32,
	/// Number of unallocated blocks in group.
	pub bg_free_blocks_count: u16,
	/// Number of unallocated inodes in group.
	pub bg_free_inodes_count: u16,
	/// Number of directories in group.
	pub bg_used_dirs_count: u16,
	/// Structure padding.
	pub bg_pad: [u8; 14],
}

impl BlockGroupDescriptor {
	/// Reads the `i`th block group descriptor from the given device.
	///
	/// Arguments:
	/// - `i` the id of the group descriptor to write.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	pub fn read(i: u32, superblock: &Superblock, io: &dyn DeviceIO) -> EResult<Self> {
		let blk_size = superblock.get_block_size();
		let off = (superblock.get_bgdt_offset() * blk_size as u64)
			+ (i as u64 * size_of::<Self>() as u64);
		read::<Self>(off, blk_size, io)
	}

	/// Writes the current block group descriptor.
	///
	/// Arguments:
	/// - `i` the id of the group descriptor to write.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	pub fn write(&self, i: u32, superblock: &Superblock, io: &dyn DeviceIO) -> EResult<()> {
		let blk_size = superblock.get_block_size();
		let off = (superblock.get_bgdt_offset() * blk_size as u64)
			+ (i as u64 * size_of::<Self>() as u64);
		write(off, blk_size, io, self)
	}
}
