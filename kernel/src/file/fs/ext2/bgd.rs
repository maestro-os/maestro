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

use super::{blk_to_page, Superblock, SUPERBLOCK_OFFSET};
use crate::device::BlkDev;
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

	pub bg_pad: [u8; 14],
}

impl BlockGroupDescriptor {
	/// Returns the `i`th block group descriptor
	pub fn get(i: u32, sp: &Superblock, dev: &BlkDev) -> EResult<RcPageObj<Self>> {
		let blk_size = sp.get_block_size() as usize;
		let bgd_per_blk = blk_size / size_of::<Self>();
		let bgdt_blk = (SUPERBLOCK_OFFSET / blk_size) + 1;
		let blk = bgdt_blk + (i as usize / bgd_per_blk);
		let page = dev.read_page(blk_to_page(blk as _, blk_size as _))?;
		Ok(RcPageObj::new(page, i as usize % bgd_per_blk))
	}
}
