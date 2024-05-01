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

//! An inode represents a file in the filesystem.

use super::{
	block_group_descriptor::BlockGroupDescriptor, dirent, dirent::Dirent, read, read_block, write,
	write_block, zero_blocks, Superblock,
};
use crate::file::{FileType, Mode};
use core::{
	cmp::{max, min},
	intrinsics::unlikely,
	mem,
	mem::size_of,
	num::{NonZeroU16, NonZeroU32},
};
use macros::AnyRepr;
use utils::{boxed::Box, errno, errno::EResult, io::IO, vec};

/// The maximum number of direct blocks for each inodes.
pub const DIRECT_BLOCKS_COUNT: usize = 12;

/// INode type: FIFO
pub const INODE_TYPE_FIFO: u16 = 0x1000;
/// INode type: Char device
pub const INODE_TYPE_CHAR_DEVICE: u16 = 0x2000;
/// INode type: Directory
pub const INODE_TYPE_DIRECTORY: u16 = 0x4000;
/// INode type: Block device
pub const INODE_TYPE_BLOCK_DEVICE: u16 = 0x6000;
/// INode type: Regular file
pub const INODE_TYPE_REGULAR: u16 = 0x8000;
/// INode type: Symbolic link
pub const INODE_TYPE_SYMLINK: u16 = 0xa000;
/// INode type: Socket
pub const INODE_TYPE_SOCKET: u16 = 0xc000;

/// User: Read, Write and Execute.
const INODE_PERMISSION_IRWXU: u16 = 0o0700;
/// User: Read.
const INODE_PERMISSION_IRUSR: u16 = 0o0400;
/// User: Write.
const INODE_PERMISSION_IWUSR: u16 = 0o0200;
/// User: Execute.
const INODE_PERMISSION_IXUSR: u16 = 0o0100;
/// Group: Read, Write and Execute.
const INODE_PERMISSION_IRWXG: u16 = 0o0070;
/// Group: Read.
const INODE_PERMISSION_IRGRP: u16 = 0o0040;
/// Group: Write.
const INODE_PERMISSION_IWGRP: u16 = 0o0020;
/// Group: Execute.
const INODE_PERMISSION_IXGRP: u16 = 0o0010;
/// Other: Read, Write and Execute.
const INODE_PERMISSION_IRWXO: u16 = 0o0007;
/// Other: Read.
const INODE_PERMISSION_IROTH: u16 = 0o0004;
/// Other: Write.
const INODE_PERMISSION_IWOTH: u16 = 0o0002;
/// Other: Execute.
const INODE_PERMISSION_IXOTH: u16 = 0o0001;
/// Setuid.
const INODE_PERMISSION_ISUID: u16 = 0o4000;
/// Setgid.
const INODE_PERMISSION_ISGID: u16 = 0o2000;
/// Sticky bit.
const INODE_PERMISSION_ISVTX: u16 = 0o1000;

/// `s_flags`: Secure deletion
const INODE_FLAG_SECURE_DELETION: u32 = 0x00001;
/// `s_flags`: Keep a copy of data when deleted
const INODE_FLAG_DELETE_COPY: u32 = 0x00002;
/// `s_flags`: File compression
const INODE_FLAG_COMPRESSION: u32 = 0x00004;
/// `s_flags`: Synchronous updates
const INODE_FLAG_SYNC: u32 = 0x00008;
/// `s_flags`: Immutable file
const INODE_FLAG_IMMUTABLE: u32 = 0x00010;
/// `s_flags`: Append only
const INODE_FLAG_APPEND_ONLY: u32 = 0x00020;
/// `s_flags`: File is not included in 'dump' command
const INODE_FLAG_NODUMP: u32 = 0x00040;
/// `s_flags`: Last accessed time should not be updated
const INODE_FLAG_ATIME_NOUPDATE: u32 = 0x00080;
/// `s_flags`: Hash indexed directory
const INODE_FLAG_HASH_INDEXED: u32 = 0x10000;
/// `s_flags`: AFS directory
const INODE_FLAG_AFS_DIRECTORY: u32 = 0x20000;
/// `s_flags`: Journal file data
const INODE_FLAG_JOURNAL_FILE: u32 = 0x40000;

/// The size of a sector in bytes.
const SECTOR_SIZE: u32 = 512;

/// The limit length for a symlink to be stored in the inode itself instead of a
/// separate block.
const SYMLINK_INODE_STORE_LIMIT: u64 = 60;

/// The inode of the root directory.
pub const ROOT_DIRECTORY_INODE: u32 = 2;
/// The root directory's default mode.
pub const ROOT_DIRECTORY_DEFAULT_MODE: u16 = INODE_PERMISSION_IRWXU
	| INODE_PERMISSION_IRGRP
	| INODE_PERMISSION_IXGRP
	| INODE_PERMISSION_IROTH
	| INODE_PERMISSION_IXOTH;

/// An inode represents a file in the filesystem.
///
/// The name of the file is not included in the inode but in the directory entry associated with it
/// since several entries can refer to the same inode (hard links).
#[repr(C, packed)]
#[derive(AnyRepr)]
pub struct Ext2INode {
	/// Type and permissions.
	pub i_mode: u16,
	/// User ID.
	pub i_uid: u16,
	/// Lower 32 bits of size in bytes.
	pub i_size: u32,
	/// Timestamp of the last modification of the metadata.
	pub i_ctime: u32,
	/// Timestamp of the last modification of the content.
	pub i_mtime: u32,
	/// Timestamp of the last access.
	pub i_atime: u32,
	/// Timestamp of the deletion.
	pub i_dtime: u32,
	/// Group ID.
	pub i_gid: u16,
	/// The number of hard links to this inode.
	pub i_links_count: u16,
	/// The number of sectors used by this inode.
	pub i_blocks: u32,
	/// INode flags.
	pub i_flags: u32,
	/// OS-specific value.
	pub i_osd1: u32,
	/// Direct block pointers.
	pub i_block: [u32; DIRECT_BLOCKS_COUNT + 3],
	/// Generation number.
	pub i_generation: u32,
	/// The file's ACL.
	pub i_file_acl: u32,
	/// Higher 32 bits of size in bytes.
	pub i_dir_acl: u32,
	/// Block address of fragment.
	pub i_faddr: u32,
	/// OS-specific value.
	pub i_osd2: [u8; 12],
}

impl Ext2INode {
	/// Returns the offset of the inode on the disk in bytes.
	///
	/// Arguments:
	/// - `i` is the inode's index (starting at `1`).
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	fn get_disk_offset(i: u32, superblock: &Superblock, io: &mut dyn IO) -> EResult<u64> {
		// Check the inode is correct
		if i == 0 {
			return Err(errno!(EINVAL));
		}
		let i = i - 1;
		let blk_size = superblock.get_block_size() as u64;
		let inode_size = superblock.get_inode_size() as u64;
		// The block group the inode is located in
		let blk_grp = i / superblock.s_inodes_per_group;
		// The offset of the inode in the block group's bitfield
		let inode_grp_off = i % superblock.s_inodes_per_group;
		// The offset of the inode's block
		let inode_table_blk_off = (inode_grp_off as u64 * inode_size) / blk_size;
		// The offset of the inode in the block
		let inode_blk_off = (i as u64 * inode_size) % blk_size;
		// Read BGD
		let bgd = BlockGroupDescriptor::read(blk_grp, superblock, io)?;
		// The block containing the inode
		let blk = bgd.bg_inode_table as u64 + inode_table_blk_off;
		// The offset of the inode on the disk
		let inode_offset = (blk * blk_size) + inode_blk_off;
		Ok(inode_offset)
	}

	/// Reads the `i`th inode from the given device.
	///
	/// Arguments:
	/// - `i` is the inode's index (starting at `1`).
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	pub fn read(i: u32, superblock: &Superblock, io: &mut dyn IO) -> EResult<Self> {
		let off = Self::get_disk_offset(i, superblock, io)?;
		read::<Self>(off, io)
	}

	/// Returns the type of the file.
	pub fn get_type(&self) -> FileType {
		let file_type = self.i_mode & 0xf000;
		match file_type {
			INODE_TYPE_FIFO => FileType::Fifo,
			INODE_TYPE_CHAR_DEVICE => FileType::CharDevice,
			INODE_TYPE_DIRECTORY => FileType::Directory,
			INODE_TYPE_BLOCK_DEVICE => FileType::BlockDevice,
			INODE_TYPE_REGULAR => FileType::Regular,
			INODE_TYPE_SYMLINK => FileType::Link,
			INODE_TYPE_SOCKET => FileType::Socket,
			_ => FileType::Regular,
		}
	}

	/// Returns the permissions of the file.
	pub fn get_permissions(&self) -> Mode {
		self.i_mode as Mode & 0x0fff
	}

	/// Sets the permissions of the file.
	pub fn set_permissions(&mut self, perm: Mode) {
		self.i_mode = (self.i_mode & !0o7777) | (perm & 0o7777) as u16;
	}

	/// Returns the size of the file.
	///
	/// `superblock` is the filesystem's superblock.
	pub fn get_size(&self, superblock: &Superblock) -> u64 {
		let has_version = superblock.s_rev_level >= 1;
		let has_feature = superblock.s_feature_ro_compat & super::WRITE_REQUIRED_64_BITS != 0;
		if has_version && has_feature {
			((self.i_dir_acl as u64) << 32) | (self.i_size as u64)
		} else {
			self.i_size as u64
		}
	}

	/// Sets the file's size.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `size` is the file's size.
	fn set_size(&mut self, superblock: &Superblock, size: u64) {
		let has_version = superblock.s_rev_level >= 1;
		let has_feature = superblock.s_feature_ro_compat & super::WRITE_REQUIRED_64_BITS != 0;
		if has_version && has_feature {
			self.i_dir_acl = ((size >> 32) & 0xffffffff) as u32;
			self.i_size = (size & 0xffffffff) as u32;
		} else {
			self.i_size = size as u32;
		}
	}

	/// Increments the number of used sectors of one block.
	///
	/// `blk_size` is the size of a block.
	fn increment_used_sectors(&mut self, blk_size: u32) {
		// The block size is a multiple of the sector size
		self.i_blocks += blk_size / SECTOR_SIZE;
	}

	/// Decrements the number of used sectors of one block.
	///
	/// `blk_size` is the size of a block.
	fn decrement_used_sectors(&mut self, blk_size: u32) {
		// The block size is a multiple of the sector size
		self.i_blocks -= blk_size / SECTOR_SIZE;
	}

	/// Translates the given file block offset `off` to disk block offset.
	///
	/// Arguments:
	/// - `off` is the file block offset
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	///
	/// If the block does not exist, the function returns `None`.
	fn translate_blk_offset(
		&self,
		mut off: u32,
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<Option<NonZeroU32>> {
		// If not indirection is required, stop here
		if off < DIRECT_BLOCKS_COUNT as _ {
			let blk = self.i_block[off as usize];
			if blk >= superblock.s_blocks_count {
				return Err(errno!(EUCLEAN));
			}
			return Ok(NonZeroU32::new(blk));
		}
		off -= DIRECT_BLOCKS_COUNT as u32;
		/*
		 * blk_size = 2^^(superblock.s_log_block_size + 10)
		 * ent_size = 4
		 * ent_per_blk = blk_size / ent_size
		 * ent_per_blk_log = log_2(ent_per_blk)
		 *                 = log_2(blk_size) - log_2(ent_size)
		 *                 = log_2(blk_size) - 2
		 *
		 * Let `n` be the number of indirections
		 *
		 * n = log_(ent_per_blk)(off)
		 *   = log_2(off) / log_2(ent_per_blk)
		 *   = log_2(off) / ent_per_blk_log
		 */
		let ent_per_blk_log = (superblock.s_log_block_size + 10) - 2;
		let n = off.checked_ilog2().unwrap_or(0) / ent_per_blk_log;
		let mut blk = self.i_block[DIRECT_BLOCKS_COUNT + n as usize];
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0u8; blk_size as _]?;
		for n in (0..=n).rev() {
			if blk == 0 {
				break;
			}
			if unlikely(blk >= superblock.s_blocks_count) {
				return Err(errno!(EUCLEAN));
			}
			read_block(blk, superblock, io, &mut buf)?;
			/*
			 * inner_off = off / (ent_per_blk * 2^^n)
			 *           = off / 2^^(ent_per_blk_log + n)
			 */
			let inner_off = (off >> (ent_per_blk_log + n)) as usize;
			blk = u32::from_le_bytes([
				buf[inner_off + 0],
				buf[inner_off + 1],
				buf[inner_off + 2],
				buf[inner_off + 3],
			]);
		}
		// No need to check correctness of `blk` as the loop has at least one iteration
		Ok(NonZeroU32::new(blk))
	}

	/// Allocates a new block for the content of the file through block
	/// indirections.
	///
	/// Arguments:
	/// - `n` is the number of indirections to resolve.
	/// - `begin` is the beginning block.
	/// - `off` is the offset of the block relative to the specified beginning
	/// block.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// The function returns the allocated block.
	fn indirections_alloc(
		&mut self,
		n: usize,
		begin: u32,
		off: u32,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<u32> {
		if begin >= superblock.s_blocks_count {
			return Err(errno!(EUCLEAN));
		}
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;
		if n > 0 {
			let blk_per_blk = entries_per_blk.pow((n - 1) as _);
			let inner_index = off / blk_per_blk;
			let inner_off = inner_index as u64 * size_of::<u32>() as u64;
			debug_assert!(inner_off < blk_size as u64);
			let byte_off = (begin as u64 * blk_size as u64) + inner_off;
			let mut b = read::<u32>(byte_off, io)?;
			if b == 0 {
				let blk = superblock.get_free_block(io)?;
				superblock.mark_block_used(io, blk)?;
				superblock.write(io)?;
				zero_blocks(blk as _, 1, superblock, io)?;
				write::<u32>(&blk, byte_off, io)?;
				self.increment_used_sectors(blk_size);
				b = blk;
			}
			let next_off = off - blk_per_blk * inner_index;
			self.indirections_alloc(n - 1, b, next_off, superblock, io)
		} else {
			Ok(begin)
		}
	}

	/// Allocates a block for the node's content block at the given offset `i`.
	/// If the block is already allocated, the function does nothing.
	///
	/// Arguments:
	/// - `i` is the block offset in the node's content.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// On success, the function returns the allocated final block offset.
	fn alloc_content_block(
		&mut self,
		i: u32,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<u64> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;
		// The number of indirections to perform
		let level = Self::indirections_count(i, entries_per_blk);
		// If direct block, handle it directly
		if level == 0 {
			let blk = superblock.get_free_block(io)?;
			superblock.mark_block_used(io, blk)?;
			superblock.write(io)?;
			zero_blocks(blk as _, 1, superblock, io)?;
			self.i_block[i as usize] = blk;
			self.increment_used_sectors(blk_size);
			return Ok(blk);
		}
		// The id on the beginning block to indirect from
		let begin_id = Self::blk_offset_to_option(self.i_block[DIRECT_BLOCKS_COUNT + level - 1]);
		let target = i - DIRECT_BLOCKS_COUNT as u32 - {
			match level {
				1 => 0,
				2 => entries_per_blk,
				3 => entries_per_blk * entries_per_blk,
				_ => unreachable!(),
			}
		};
		if let Some(begin_id) = begin_id {
			self.indirections_alloc(level, begin_id, target, superblock, io)
		} else {
			let begin = superblock.get_free_block(io)?;
			superblock.mark_block_used(io, begin)?;
			superblock.write(io)?;
			zero_blocks(begin as _, 1, superblock, io)?;
			self.i_block[DIRECT_BLOCKS_COUNT + level - 1] = begin;
			self.increment_used_sectors(blk_size);
			self.indirections_alloc(level, begin, target, superblock, io)
		}
	}

	/// Tells whether the given block has all its entries empty.
	fn is_blk_empty(blk: &[u8]) -> bool {
		// The block size will always be a power of two and higher than `8`
		blk.array_chunks::<8>().all(|b| u64::from_ne_bytes(*b) == 0)
	}

	/// Frees a block of the content of the file through block indirections.
	///
	/// Arguments:
	/// - `n` is the number of indirections to resolve.
	/// - `begin` is the beginning block.
	/// - `off` is the offset of the block relative to the specified beginning
	/// block.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// The function returns a boolean telling whether the block at `begin` has
	/// been freed.
	fn indirections_free(
		&mut self,
		n: usize,
		begin: u32,
		off: u32,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<bool> {
		if begin >= superblock.s_blocks_count {
			return Err(errno!(EUCLEAN));
		}
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;
		if n > 0 {
			let blk_per_blk = entries_per_blk.pow((n - 1) as _);
			let inner_index = off / blk_per_blk;
			let inner_off = inner_index as u64 * size_of::<u32>() as u64;
			debug_assert!(inner_off < blk_size as u64);
			let byte_off = (begin as u64 * blk_size as u64) + inner_off;
			let b = read::<u32>(byte_off, io)?;
			let next_off = off - blk_per_blk * inner_index;
			if self.indirections_free(n - 1, b, next_off, superblock, io)? {
				// Reading the current block
				let mut buff = vec![0; blk_size as _]?;
				read_block(begin as _, superblock, io, buff.as_mut_slice())?;
				// If the current block is empty, free it
				if Self::is_blk_empty(buff.as_slice()) {
					superblock.free_block(io, begin)?;
					self.decrement_used_sectors(blk_size);
					return Ok(true);
				}
			}
			Ok(false)
		} else {
			superblock.free_block(io, begin)?;
			Ok(true)
		}
	}

	/// Frees a content block at block offset `i` in file.
	/// If the block isn't allocated, the function does nothing.
	///
	/// Arguments:
	/// - `i` is the id of the block.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	fn free_content_block(
		&mut self,
		i: u32,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<()> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;
		// The number of indirections to perform
		let level = Self::indirections_count(i, entries_per_blk);
		// If direct block, handle it directly
		if level == 0 {
			superblock.free_block(io, self.i_block[i as usize])?;
			self.i_block[i as usize] = 0;
			self.decrement_used_sectors(blk_size);
			return Ok(());
		}
		// The id on the beginning block to indirect from
		let begin_id = Self::blk_offset_to_option(self.i_block[DIRECT_BLOCKS_COUNT + level - 1]);
		let target = i - DIRECT_BLOCKS_COUNT as u32 - {
			match level {
				1 => 0,
				2 => entries_per_blk,
				3 => entries_per_blk * entries_per_blk,
				_ => unreachable!(),
			}
		};
		if let Some(begin_id) = begin_id {
			let empty = self.indirections_free(level, begin_id, target, superblock, io)?;
			// If the block has zero entries left, free it
			if empty {
				superblock.free_block(io, begin_id)?;
				self.i_block[DIRECT_BLOCKS_COUNT + level - 1] = 0;
				self.decrement_used_sectors(blk_size);
			}
		}
		Ok(())
	}

	/// Reads the content of the inode.
	///
	/// Arguments:
	/// - `off` is the offset at which the inode is read.
	/// - `buff` is the buffer in which the data is to be written.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// The function returns the number of bytes that have been read and boolean
	/// telling whether EOF is reached.
	pub fn read_content(
		&self,
		off: u64,
		buff: &mut [u8],
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<(u64, bool)> {
		let size = self.get_size(superblock);
		if off > size {
			return Err(errno!(EINVAL));
		}

		let blk_size = superblock.get_block_size();
		let mut blk_buff = vec![0u8; blk_size as _]?;

		let mut i = 0;
		let max = min(buff.len() as u64, size - off);
		while i < max {
			let blk_off = (off + i) / blk_size as u64;
			let blk_inner_off = ((off + i) % blk_size as u64) as usize;
			let len = min(max - i, (blk_size - blk_inner_off as u32) as u64);

			let dst = &mut buff[(i as usize)..((i + len) as usize)];

			if let Some(blk_off) = self.translate_blk_offset(blk_off as _, superblock, io)? {
				read_block(blk_off as _, superblock, io, blk_buff.as_mut_slice())?;

				let src = &blk_buff.as_slice()[blk_inner_off..(blk_inner_off + len as usize)];
				dst.copy_from_slice(src);
			} else {
				// No content block, writing zeros
				dst.fill(0);
			}

			i += len;
		}
		let eof = off + i >= size;
		Ok((i, eof))
	}

	/// Writes the content of the inode.
	///
	/// Arguments:
	/// - `off` is the offset at which the inode is written.
	/// - `buff` is the buffer in which the data is to be written.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// The function returns the number of bytes that have been written.
	pub fn write_content(
		&mut self,
		off: u64,
		buff: &[u8],
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<()> {
		let curr_size = self.get_size(superblock);
		if off > curr_size {
			return Err(errno!(EINVAL));
		}
		let blk_size = superblock.get_block_size();
		let mut blk_buff = vec![0u8; blk_size as _]?;
		let mut i = 0;
		while i < buff.len() {
			// Get block offset and read it
			let blk_off = (off + i as u64) / blk_size as u64;
			let blk_off =
				if let Some(blk_off) = self.translate_blk_offset(blk_off as _, superblock, io)? {
					read_block(blk_off as _, superblock, io, &mut blk_buff)?;
					blk_off
				} else {
					blk_buff.fill(0);
					self.alloc_content_block(blk_off as u32, superblock, io)?
				};
			// Offset inside the block
			let blk_inner_off = ((off + i as u64) % blk_size as u64) as usize;
			// Write data to buffer
			let len = min(buff.len() - i, (blk_size - blk_inner_off as u32) as usize);
			blk_buff[blk_inner_off..(blk_inner_off + len)].copy_from_slice(&buff[i..(i + len)]);
			// Write block
			write_block(blk_off as _, superblock, io, &blk_buff)?;
			i += len;
		}
		// Update size
		let new_size = max(off + buff.len() as u64, curr_size);
		self.set_size(superblock, new_size);
		Ok(())
	}

	/// Truncates the file to the given size `size`.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	/// - `size` is the new size of the inode's content.
	///
	/// If `size` is greater than or equal to the previous size, the function
	/// does nothing.
	pub fn truncate(
		&mut self,
		superblock: &mut Superblock,
		io: &mut dyn IO,
		size: u64,
	) -> EResult<()> {
		let old_size = self.get_size(superblock);
		if size >= old_size {
			return Ok(());
		}
		// Change the size
		self.set_size(superblock, size);
		// The size of a block
		let blk_size = superblock.get_block_size();
		// The index of the beginning block to free
		let begin = size.div_ceil(blk_size as _) as u32;
		// The index of the end block to free
		let end = old_size.div_ceil(blk_size as _) as u32;
		for i in begin..end {
			self.free_content_block(i, superblock, io)?;
		}
		Ok(())
	}

	/// Frees all content blocks by doing redirections.
	///
	/// Arguments:
	/// - `begin` is the beginning block.
	/// - `n` is the number of indirections.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	fn indirect_free_all(
		begin: u32,
		n: usize,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<()> {
		if begin >= superblock.s_blocks_count {
			return Err(errno!(EUCLEAN));
		}
		let blk_size = superblock.get_block_size();
		// Read the block
		let mut blk_buff = vec![0; blk_size as _]?;
		read_block(begin as _, superblock, io, blk_buff.as_mut_slice())?;
		// Free every entry recursively
		if n > 0 {
			let entries_per_blk = blk_size as usize / size_of::<u32>();
			for i in 0..entries_per_blk {
				let b = u32::from_le_bytes([
					blk_buff[i],
					blk_buff[i + 1],
					blk_buff[i + 2],
					blk_buff[i + 3],
				]);
				// If the entry is not empty, free it
				if b != 0 {
					Self::indirect_free_all(b, n - 1, superblock, io)?;
				}
			}
		}
		superblock.free_block(io, begin)
	}

	/// Frees all the content blocks of the inode.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	pub fn free_content(&mut self, superblock: &mut Superblock, io: &mut dyn IO) -> EResult<()> {
		if matches!(self.get_type(), FileType::Link) {
			let len = self.get_size(superblock);
			if len <= SYMLINK_INODE_STORE_LIMIT {
				return Ok(());
			}
		}

		for i in 0..DIRECT_BLOCKS_COUNT {
			if self.i_block[i] != 0 {
				if self.i_block[i] >= superblock.s_blocks_count {
					return Err(errno!(EUCLEAN));
				}
				superblock.free_block(io, self.i_block[i])?;
				self.i_block[i] = 0;
			}
		}

		if self.i_block[DIRECT_BLOCKS_COUNT] != 0 {
			Self::indirect_free_all(self.i_block[DIRECT_BLOCKS_COUNT], 1, superblock, io)?;
			self.i_block[DIRECT_BLOCKS_COUNT] = 0;
		}
		if self.i_block[DIRECT_BLOCKS_COUNT + 1] != 0 {
			Self::indirect_free_all(self.i_block[DIRECT_BLOCKS_COUNT + 1], 2, superblock, io)?;
			self.i_block[DIRECT_BLOCKS_COUNT + 1] = 0;
		}
		if self.i_block[DIRECT_BLOCKS_COUNT + 2] != 0 {
			Self::indirect_free_all(self.i_block[DIRECT_BLOCKS_COUNT + 2], 3, superblock, io)?;
			self.i_block[DIRECT_BLOCKS_COUNT + 2] = 0;
		}

		// Update the number of used sectors
		self.i_blocks = 0;

		Ok(())
	}

	/// Returns the directory entry with the given name `name`, along with the offset of the entry.
	///
	/// Arguments:
	/// - `name` is the name of the entry.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// If the entry doesn't exist, the function returns `None`.
	///
	/// If the file is not a directory, the function returns `None`.
	pub fn get_dirent(
		&self,
		name: &[u8],
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<Option<(Box<Dirent>, u64)>> {
		// TODO If the hash index is enabled, use it
		let mut off = 0;
		while let Some((ent, next_off)) = self.next_dirent(off, superblock, io)? {
			if !ent.is_free() && ent.get_name(superblock) == name {
				return Ok(Some((ent, off)));
			}
			off = next_off;
		}
		Ok(None)
	}

	/// Returns the directory entry at offset `off`.
	///
	/// Arguments:
	/// - `off` is the offset of the entry to return.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// On success, the function returns the entry and the offset to the next entry.
	pub fn next_dirent(
		&self,
		off: u64,
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<Option<(Box<Dirent>, u64)>> {
		if self.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// TODO If the binary tree feature is enabled, use it
		// If the list is exhausted, stop
		if off >= self.get_size(superblock) {
			return Ok(None);
		}
		// Read the entry
		let ent = self.read_dirent(off, superblock, io)?;
		// `rec_len` has been checked when reading the entry. It will never be zero
		let next_off = off.saturating_add(ent.rec_len as _);
		Ok(Some((ent, next_off)))
	}

	/// Looks for an entry in the inode which is large enough to fit
	/// another entry with the given size.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	/// - `min_size` is the minimum size of the new entry in bytes.
	///
	/// If the function finds an entry, it returns its offset. Else, the
	/// function returns `None`.
	///
	/// If the file is not a directory, the function
	/// returns `None`.
	fn get_suitable_entry(
		&self,
		superblock: &Superblock,
		io: &mut dyn IO,
		min_size: NonZeroU16,
	) -> EResult<Option<u64>> {
		let mut off = 0;
		while let Some((ent, next_off)) = self.next_dirent(off, superblock, io)? {
			if ent.would_fit(min_size) {
				return Ok(Some(off));
			}
			off = next_off;
		}
		Ok(None)
	}

	/// Adds a new entry to the current directory.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	/// - `entry_inode` is the inode of the entry.
	/// - `name` is the name of the entry.
	/// - `file_type` is the type of the entry.
	///
	/// If the block allocation fails or if the entry name is already used, the
	/// function returns an error.
	///
	/// If the file is not a directory, the behaviour is undefined.
	pub fn add_dirent(
		&mut self,
		superblock: &mut Superblock,
		io: &mut dyn IO,
		entry_inode: u32,
		name: &[u8],
		file_type: FileType,
	) -> EResult<()> {
		debug_assert_eq!(self.get_type(), FileType::Directory);
		// If the name is too long, error
		if name.len() > super::MAX_NAME_LEN {
			return Err(errno!(ENAMETOOLONG));
		}
		let rec_len: NonZeroU16 = ((dirent::NAME_OFF + name.len()) as u16)
			// cannot overflow thanks to the previous check
			.next_multiple_of(dirent::ALIGN as u16)
			.try_into()
			// cannot fail because the value cannot be zero
			.unwrap();
		// If the entry is too large, error
		let blk_size = superblock.get_block_size();
		if rec_len.get() as u32 > blk_size {
			return Err(errno!(ENAMETOOLONG));
		}
		if let Some(entry_off) = self.get_suitable_entry(superblock, io, rec_len)? {
			let mut entry = self.read_dirent(entry_off, superblock, io)?;
			// TODO this is dirty. merge everything in one place
			// TODO when using a free entry, must create a free entry after the new to cover the
			// remaining free space
			let (mut new_entry, new_entry_off) = entry.insert(superblock, rec_len)?;
			new_entry.inode = entry_inode;
			new_entry.set_name(superblock, name)?;
			new_entry.set_type(superblock, file_type);
			// Save the new entry first to make the operation atomic: if the second operation
			// fails, the new entry remains hidden
			self.write_dirent(superblock, io, &new_entry, entry_off + new_entry_off)?;
			// Do not rewrite new entry with previous
			if new_entry_off > 0 {
				self.write_dirent(superblock, io, &entry, entry_off)?;
			}
		} else {
			// No suitable free entry: Allocate and fill a new block
			let mut blk = vec![0u8; blk_size as _]?;
			// Create used entry
			let entry = Dirent::new(superblock, entry_inode, rec_len, file_type, name)?;
			// TODO copy to blk. or create onto blk directly?
			// Create free entries to cover remaining free space
			let mut off = entry.rec_len as usize;
			while off < blk.len() {
				// TODO create free entry
			}
			let off = (self.get_size(superblock) / blk_size as u64) as u32;
			// TODO function zeros the block. this is unnecessary
			let off = self.alloc_content_block(off, superblock, io)?;
			write_block(off, superblock, io, &blk)?;
		}
		Ok(())
	}

	/// Finds the previous directory entry *in the same block* from the given offset `off`.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	///
	/// If there is no previous entry, the function returns the entry at `off`.
	///
	/// On success, the function returns the previous directory entry along with its offset.
	fn prev_block_dirent(
		&self,
		off: u64,
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<(Box<Dirent>, u64)> {
		let blk_size = superblock.get_block_size() as u64;
		// Initialize the offset to the beginning of the current block
		let mut o = off - (off % blk_size);
		while let Some((ent, next_off)) = self.next_dirent(o, superblock, io)? {
			if next_off == off {
				return Ok((ent, o));
			}
			o = next_off;
		}
		Err(errno!(EUCLEAN))
	}

	/// Removes the entry from the current directory.
	///
	/// Arguments:
	/// - `off` is the offset of the entry to remove.
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	pub fn remove_dirent(
		&mut self,
		off: u64,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<()> {
		debug_assert_eq!(self.get_type(), FileType::Directory);
		let blk_size = superblock.get_block_size() as u64;
		let size = self.get_size(superblock);
		// Free entry
		let mut ent = self.read_dirent(off, superblock, io)?;
		ent.inode = 0;
		// If the next entry is free, merge
		let next_off = off + ent.rec_len as u64;
		if next_off < size {
			let next_ent = self.read_dirent(next_off, superblock, io)?;
			if next_ent.is_free() {
				ent.merge(next_ent);
			}
		}
		// Find the offset of the previous entry
		let (prev_ent, prev_off) = self.prev_block_dirent(off, superblock, io)?;
		// If there is a previous entry, and it is free: merge `ent` into it
		if prev_off < off && prev_ent.is_free() {
			let old = mem::replace(&mut ent, prev_ent);
			// TODO: what if this would overflow `rec_len`?
			ent.merge(old);
		}
		// If this is the last block, and it is now empty, free it
		let next_blk_off = off.next_multiple_of(blk_size);
		if next_blk_off >= size && ent.rec_len as u64 >= blk_size {
			let cur_blk_off = off - (off % blk_size);
			let cur_blk = (cur_blk_off / blk_size) as u32;
			self.set_size(superblock, cur_blk_off);
			// FIXME: consistency: need to update the inode *before* freeing the block to avoid
			// dangling references
			self.free_content_block(cur_blk, superblock, io)?;
		}
		self.write_dirent(superblock, io, &ent, off)
	}

	/// Reads the content symbolic link.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	/// - `off` is the offset from which the link is read.
	/// - `buf` is the buffer in which the content is written.
	///
	/// If the file is not a symbolic link, the behaviour is undefined.
	///
	/// On success, the function returns the number of bytes written to `buf`.
	pub fn read_link(
		&self,
		superblock: &Superblock,
		io: &mut dyn IO,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		let size = self.get_size(superblock);
		if size <= SYMLINK_INODE_STORE_LIMIT {
			// The target is stored inline in the inode
			let copy_max = min(buf.len(), (size - off) as _);
			let buf = &mut buf[..copy_max];
			let mut i = 0;
			self.i_block
				.into_iter()
				.flat_map(u32::to_le_bytes)
				.skip(off as _)
				.zip(buf.iter_mut())
				.for_each(|(src, dst)| {
					*dst = src;
					i += 1;
				});
			Ok(i)
		} else {
			// The target is stored like in regular files
			Ok(self.read_content(off, buf, superblock, io)?.0)
		}
	}

	/// Writes the content symbolic link. The function always truncates the content to the size of
	/// `buf`.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	/// - `buf` is the buffer in which the content is written.
	///
	/// If the file is not a symbolic link, the behaviour is undefined.
	pub fn write_link(
		&mut self,
		superblock: &mut Superblock,
		io: &mut dyn IO,
		buf: &[u8],
	) -> EResult<()> {
		let old_size = self.get_size(superblock);
		let new_size = buf.len() as u64;
		// Erase previous
		if old_size <= SYMLINK_INODE_STORE_LIMIT {
			// A manual loop is required because `i_block` is potentially unaligned
			for i in 0..(DIRECT_BLOCKS_COUNT + 3) {
				self.i_block[i] = 0;
			}
		}
		// Write target
		if new_size <= SYMLINK_INODE_STORE_LIMIT {
			// The target is stored inline in the inode
			self.truncate(superblock, io, 0)?;
			// A manual loop is required because `i_block` is potentially unaligned
			for (i, b) in buf.iter().enumerate() {
				self.i_block[i / 4] |= (*b as u32) << (i % 4);
			}
			self.set_size(superblock, new_size);
		} else {
			self.truncate(superblock, io, new_size)?;
			self.write_content(0, buf, superblock, io)?;
		}
		Ok(())
	}

	/// Returns the device major and minor numbers associated with the device.
	///
	/// If the file is not a device file, the function returns `(0, 0)`.
	pub fn get_device(&self) -> (u8, u8) {
		match self.get_type() {
			FileType::BlockDevice | FileType::CharDevice => {
				let dev = self.i_block[0];
				(((dev >> 8) & 0xff) as u8, (dev & 0xff) as u8)
			}
			_ => (0, 0),
		}
	}

	/// Sets the device major and minor numbers associated with the device.
	///
	/// Arguments:
	/// - `major` is the major number.
	/// - `minor` is the minor number.
	///
	/// If the file is not a device file, the function does nothing.
	pub fn set_device(&mut self, major: u8, minor: u8) {
		if matches!(
			self.get_type(),
			FileType::BlockDevice | FileType::CharDevice
		) {
			self.i_block[0] = ((major as u32) << 8) | (minor as u32);
		}
	}

	/// Writes the inode on the device.
	pub fn write(&self, i: u32, superblock: &Superblock, io: &mut dyn IO) -> EResult<()> {
		let off = Self::get_disk_offset(i, superblock, io)?;
		write(self, off, io)
	}
}
