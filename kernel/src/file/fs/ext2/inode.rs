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
	write_block, Superblock,
};
use crate::file::{DirEntry, FileType, Mode};
use core::{
	cmp::{max, min},
	intrinsics::unlikely,
	num::{NonZeroU16, NonZeroU32},
};
use macros::AnyRepr;
use utils::{errno, errno::EResult, io::IO, math, ptr::cow::Cow, vec};

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

/// Returns a tuple containing:
/// - The number of indirections for the given `off`
/// - The updated value of `off` for it to be relative to the corresponding `i_block` slot
///
/// If no indirection is necessary, the function returns `None`.
///
/// If the offset is out of bounds, the function returns [`EOVERFLOW`].
fn indirections_count(mut off: u32, superblock: &Superblock) -> EResult<(Option<u32>, u32)> {
	if off < DIRECT_BLOCKS_COUNT as u32 {
		return Ok((None, off));
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
	let indir_count = off.checked_ilog2().unwrap_or(0) / ent_per_blk_log;
	// Get first block with bound check
	if unlikely(indir_count >= 3) {
		return Err(errno!(EOVERFLOW));
	}
	// Adapt offset
	for n in 0..indir_count {
		off -= math::pow2(ent_per_blk_log * n);
	}
	Ok((Some(indir_count), off))
}

/// Checks for an invalid block number.
///
/// If the block number is zero, the function returns `None`.
fn check_blk_off(blk: u32, superblock: &Superblock) -> EResult<Option<NonZeroU32>> {
	if unlikely(blk >= superblock.s_blocks_count) {
		return Err(errno!(EUCLEAN));
	}
	Ok(NonZeroU32::new(blk))
}

/// Returns the inner offset in the indirection block for the given offset `off` and indirection
/// level `level.
fn indirection_inner_off(off: u32, level: u32, superblock: &Superblock) -> usize {
	let ent_per_blk_log = (superblock.s_log_block_size + 10) - 2;
	/*
	 * inner_off = off / ent_per_blk^^n
	 *           = off / 2^^(ent_per_blk_log * n)
	 */
	(off >> (ent_per_blk_log * level)) as usize
}

/// Loads an index from an indirection block.
fn indirection_load(
	buf: &[u8],
	off: u32,
	level: u32,
	superblock: &Superblock,
) -> EResult<Option<NonZeroU32>> {
	let inner_off = indirection_inner_off(off, level, superblock);
	let blk = u32::from_le_bytes([
		buf[inner_off * 4],
		buf[inner_off * 4 + 1],
		buf[inner_off * 4 + 2],
		buf[inner_off * 4 + 3],
	]);
	check_blk_off(blk, superblock)
}

/// Stores an index into an indirection block.
fn indirection_store(buf: &mut [u8], off: u32, level: u32, val: u32, superblock: &Superblock) {
	let inner_off = indirection_inner_off(off, level, superblock);
	let val_arr = val.to_le_bytes();
	buf[inner_off * 4..(inner_off + 1) * 4].copy_from_slice(&val_arr);
}

/// Returns the next directory entry.
///
/// Arguments:
/// - `node` is the node containing the entry
/// - `superblock` is the filesystem's superblock
/// - `io` is the I/O interface
/// - `buf` is the block buffer
/// - `off` is the beginning offset.
///
/// The [`Iterator`] trait cannot be used because of lifetime issues.
fn next_dirent<'b>(
	node: &Ext2INode,
	superblock: &Superblock,
	io: &mut dyn IO,
	buf: &'b mut [u8],
	off: &mut u64,
) -> EResult<Option<&'b mut Dirent>> {
	let blk_size = superblock.get_block_size() as u64;
	let blk_off = *off / blk_size;
	let inner_off = (*off % blk_size) as usize;
	// If at the beginning of a block, read it
	if inner_off == 0 {
		let res = node.translate_blk_off(blk_off as _, superblock, io);
		let blk_off = match res {
			Ok(Some(o)) => o,
			// If reaching a zero block, stop
			Ok(None) => return Ok(None),
			// If reaching the block limit, stop
			Err(e) if e.as_int() == errno::EOVERFLOW => return Ok(None),
			Err(e) => return Err(e),
		};
		read_block(blk_off.get() as _, superblock, io, buf)?;
	}
	let ent = Dirent::from_slice(&mut buf[inner_off..], superblock)?;
	// `rec_len` is never zero and never exceeds the remaining space of the block
	*off += ent.record_len() as u64;
	Ok(Some(ent))
}

/// Tells whether the block contains only free directory entries.
fn is_block_empty(buf: &mut [u8], superblock: &Superblock) -> EResult<bool> {
	let mut off = 0;
	while off < buf.len() {
		let ent = Dirent::from_slice(&mut buf[off..], superblock)?;
		if !ent.is_free() {
			return Ok(false);
		}
		off += ent.record_len();
	}
	Ok(true)
}

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
	fn translate_blk_off(
		&self,
		off: u32,
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<Option<NonZeroU32>> {
		let (indir_cnt, off) = indirections_count(off, superblock)?;
		let indir_cnt = match indir_cnt {
			Some(indir_cnt) => indir_cnt,
			// No indirection is required, stop here
			None => return check_blk_off(self.i_block[off as usize], superblock),
		};
		let blk = self.i_block[DIRECT_BLOCKS_COUNT + indir_cnt as usize];
		let Some(mut blk) = check_blk_off(blk, superblock)? else {
			return Ok(None);
		};
		// Perform indirections
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0u8; blk_size as _]?;
		for n in (0..=indir_cnt).rev() {
			read_block(blk.get() as _, superblock, io, &mut buf)?;
			let Some(b) = indirection_load(&buf, off, n, superblock)? else {
				return Ok(None);
			};
			blk = b;
		}
		Ok(Some(blk))
	}

	/// Allocates a block for the node's content block at the given file block offset `off`.
	///
	/// Arguments:
	/// - `off` is the file block offset at which the block is to be allocated
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	///
	/// The content of the allocated block is **not** initialized.
	///
	/// If a block is already allocated, the function does nothing.
	///
	/// On success, the function returns the allocated disk block offset.
	fn alloc_content_blk(
		&mut self,
		off: u32,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<NonZeroU32> {
		let (indir_cnt, off) = indirections_count(off, superblock)?;
		let Some(indir_cnt) = indir_cnt else {
			// No indirection is required, stop here
			let blk = check_blk_off(self.i_block[off as usize], superblock)?;
			let blk = match blk {
				Some(b) => b,
				// No block is present, allocate
				None => {
					let new_blk = superblock.get_free_block(io)?;
					superblock.mark_block_used(io, new_blk)?;
					self.i_block[off as usize] = new_blk;
					NonZeroU32::new(new_blk).unwrap()
				}
			};
			return Ok(blk);
		};
		let blk = check_blk_off(
			self.i_block[DIRECT_BLOCKS_COUNT + indir_cnt as usize],
			superblock,
		)?;
		let mut blk = match blk {
			Some(b) => b,
			// No block is present, allocate
			None => {
				let new_blk = superblock.get_free_block(io)?;
				superblock.mark_block_used(io, new_blk)?;
				self.i_block[DIRECT_BLOCKS_COUNT + indir_cnt as usize] = new_blk;
				NonZeroU32::new(new_blk).unwrap()
			}
		};
		// Perform indirections
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0u8; blk_size as _]?;
		for n in (0..=indir_cnt).rev() {
			read_block(blk.get() as _, superblock, io, &mut buf)?;
			let b = match indirection_load(&buf, off, n, superblock)? {
				Some(b) => b,
				// No block is present, allocate
				None => {
					let new_blk = superblock.get_free_block(io)?;
					superblock.mark_block_used(io, new_blk)?;
					indirection_store(&mut buf, off, n, new_blk, superblock);
					write_block(blk.get() as _, superblock, io, &buf)?;
					NonZeroU32::new(new_blk).unwrap()
				}
			};
			blk = b;
		}
		Ok(blk)
	}

	fn free_content_blk_impl(
		blk: NonZeroU32,
		off: u32,
		n: u32,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<bool> {
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0u8; blk_size as _]?;
		read_block(blk.get() as _, superblock, io, &mut buf)?;
		// If no block is present, nothing is left to do
		let Some(b) = indirection_load(&buf, off, n, superblock)? else {
			// Assuming previous calls would have freed the block if it was emptied
			return Ok(false);
		};
		// Handle child block and determine whether the entry in the current block should be freed
		let free = if n > 0 {
			Self::free_content_blk_impl(b, n - 1, off, superblock, io)?
		} else {
			true
		};
		if free {
			indirection_store(&mut buf, off, n, 0, superblock);
			// TODO determine whether using `i_blocks` for this is correct
			let empty = buf.iter().all(|b| *b == 0);
			if !empty {
				// The block is not empty, save
				write_block(blk.get() as _, superblock, io, &buf)?;
			}
			// If the block is empty, there is no point in saving it since it will be freed
			superblock.free_block(io, b.get())?;
			Ok(empty)
		} else {
			Ok(false)
		}
	}

	/// Frees a content block at the given file block offset `off`.
	///
	/// If the block is not allocated, the function does nothing.
	///
	/// Arguments:
	/// - `off` is the file block offset of the block to free
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	fn free_content_blk(
		&mut self,
		off: u32,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<()> {
		let (indir_cnt, off) = indirections_count(off, superblock)?;
		let Some(indir_cnt) = indir_cnt else {
			// No indirection is required, stop here
			let blk = check_blk_off(self.i_block[off as usize], superblock)?;
			let Some(blk) = blk else {
				return Ok(());
			};
			// TODO write inode before freeing block to avoid dangling references
			superblock.free_block(io, blk.get())?;
			self.i_block[off as usize] = 0;
			return Ok(());
		};
		let blk = check_blk_off(
			self.i_block[DIRECT_BLOCKS_COUNT + indir_cnt as usize],
			superblock,
		)?;
		// If no block is present, stop
		let Some(blk) = blk else {
			return Ok(());
		};
		// Perform indirections
		Self::free_content_blk_impl(blk, off, indir_cnt, superblock, io)?;
		Ok(())
	}

	/// Reads the content of the inode.
	///
	/// Arguments:
	/// - `off` is the offset at which the inode is read
	/// - `buff` is the buffer in which the data is to be written
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
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
		let mut cur = 0;
		let max = min(buff.len() as u64, size - off);
		while cur < max {
			// Get slice of the destination buffer corresponding to the current block
			let blk_off = (off + cur) / blk_size as u64;
			let blk_inner_off = ((off + cur) % blk_size as u64) as usize;
			let len = min(max - cur, (blk_size - blk_inner_off as u32) as u64);
			let dst = &mut buff[(cur as usize)..((cur + len) as usize)];
			// Get disk block offset
			if let Some(blk_off) = self.translate_blk_off(blk_off as _, superblock, io)? {
				// A content block is present, copy
				read_block(blk_off.get() as _, superblock, io, &mut blk_buff)?;
				let src = &blk_buff[blk_inner_off..(blk_inner_off + len as usize)];
				dst.copy_from_slice(src);
			} else {
				// No content block, writing zeros
				dst.fill(0);
			}
			cur += len;
		}
		let eof = off + cur >= size;
		Ok((cur, eof))
	}

	/// Writes the content of the inode.
	///
	/// Arguments:
	/// - `off` is the offset at which the inode is written
	/// - `buff` is the buffer in which the data is to be written
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
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
		let mut cur = 0;
		while cur < buff.len() {
			// Get block offset and read it
			let blk_off = (off + cur as u64) / blk_size as u64;
			let blk_off =
				if let Some(blk_off) = self.translate_blk_off(blk_off as _, superblock, io)? {
					// A content block is present, read it
					read_block(blk_off.get() as _, superblock, io, &mut blk_buff)?;
					blk_off
				} else {
					// No content block, allocate one
					blk_buff.fill(0);
					self.alloc_content_blk(blk_off as u32, superblock, io)?
				};
			// Offset inside the block
			let blk_inner_off = ((off + cur as u64) % blk_size as u64) as usize;
			// Write data to buffer
			let len = min(buff.len() - cur, (blk_size - blk_inner_off as u32) as usize);
			blk_buff[blk_inner_off..(blk_inner_off + len)]
				.copy_from_slice(&buff[cur..(cur + len)]);
			// Write block
			write_block(blk_off.get() as _, superblock, io, &blk_buff)?;
			cur += len;
		}
		// Update size
		let new_size = max(off + buff.len() as u64, curr_size);
		self.set_size(superblock, new_size);
		Ok(())
	}

	/// Truncates the file to the given size `size`.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	/// - `size` is the new size of the inode's content
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
			self.free_content_blk(i, superblock, io)?;
		}
		Ok(())
	}

	/// Frees all content blocks by doing redirections.
	///
	/// Arguments:
	/// - `begin` is the beginning block
	/// - `level` is the number of indirections
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	fn indirect_free_all(
		blk: NonZeroU32,
		level: usize,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<()> {
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0; blk_size as _]?;
		read_block(blk.get() as _, superblock, io, &mut buf)?;
		// Free every entry recursively
		let entries_per_blk = blk_size / 4;
		for i in 0..entries_per_blk {
			let Some(b) = indirection_load(&buf, i, 0, superblock)? else {
				continue;
			};
			if level > 0 {
				Self::indirect_free_all(b, level - 1, superblock, io)?;
			}
			superblock.free_block(io, b.get())?;
		}
		Ok(())
	}

	/// Frees all the content blocks of the inode.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	pub fn free_content(&mut self, superblock: &mut Superblock, io: &mut dyn IO) -> EResult<()> {
		// If the file is a link and its content is stored inline, there is nothing to do
		if matches!(self.get_type(), FileType::Link)
			&& self.get_size(superblock) <= SYMLINK_INODE_STORE_LIMIT
		{
			return Ok(());
		}
		// Zeros blocks in inode and write it
		let blocks = self.i_block;
		for i in 0..(DIRECT_BLOCKS_COUNT + 3) {
			self.i_block[i] = 0;
		}
		self.i_blocks = 0;
		// TODO write inode
		// Free direct blocks
		for blk in &blocks[..DIRECT_BLOCKS_COUNT] {
			let Some(blk) = check_blk_off(*blk, superblock)? else {
				continue;
			};
			superblock.free_block(io, blk.get())?;
		}
		// Free indirect blocks
		for (indir_cnt, blk) in blocks[DIRECT_BLOCKS_COUNT..].iter().enumerate() {
			let Some(blk) = check_blk_off(*blk, superblock)? else {
				continue;
			};
			Self::indirect_free_all(blk, indir_cnt, superblock, io)?;
			superblock.free_block(io, blk.get())?;
		}
		Ok(())
	}

	/// Returns the information of a directory entry with the given name `name`.
	///
	/// Arguments:
	/// - `name` is the name of the entry
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	///
	/// The function returns:
	/// - The inode
	/// - The file type
	/// - The offset of the entry
	///
	/// If the entry doesn't exist, the function returns `None`.
	///
	/// If the file is not a directory, the function returns `None`.
	pub fn get_dirent(
		&self,
		name: &[u8],
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<Option<(u32, FileType, u64)>> {
		// Validation
		if self.get_type() != FileType::Directory {
			return Ok(None);
		}
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0; blk_size as _]?;
		// TODO If the hash index is enabled, use it
		// Linear lookup
		let mut off = 0;
		loop {
			let prev_off = off;
			let Some(ent) = next_dirent(self, superblock, io, &mut buf, &mut off)? else {
				break;
			};
			if !ent.is_free() && ent.get_name(superblock) == name {
				return Ok(Some((
					ent.inode,
					ent.get_type(superblock, &mut *io)?,
					prev_off,
				)));
			}
		}
		Ok(None)
	}

	/// Returns the next used directory entry starting from the offset `off`.
	///
	/// Arguments:
	/// - `off` is the offset of the entry to return
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	///
	/// On success, the function returns the entry and the offset to the next entry.
	pub fn next_dirent(
		&self,
		off: u64,
		superblock: &Superblock,
		io: &mut dyn IO,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		if self.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// If the list is exhausted, stop
		if off >= self.get_size(superblock) {
			return Ok(None);
		}
		// Read the entry
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0; blk_size as _]?;
		let mut next_off = off;
		let Some(ent) = next_dirent(self, superblock, io, &mut buf, &mut next_off)? else {
			return Ok(None);
		};
		let entry_type = ent.get_type(superblock, io)?;
		let name = ent.get_name(superblock).try_into()?;
		let ent = DirEntry {
			inode: ent.inode as _,
			entry_type,
			name: Cow::Owned(name),
		};
		Ok(Some((ent, next_off)))
	}

	/// Looks for a sequence of free entries large enough to fit a chunk with at least `min_size`
	/// bytes, and returns the offset to its beginning and its size in bytes.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	/// - `buf` is the block buffer
	/// - `min_size` is the minimum size of the new entry in bytes
	///
	/// If no suitable sequence is found, the function returns `None`.
	fn get_suitable_slot(
		&self,
		superblock: &Superblock,
		io: &mut dyn IO,
		buf: &mut [u8],
		min_size: NonZeroU16,
	) -> EResult<Option<(u64, usize)>> {
		let blk_size = superblock.get_block_size() as u64;
		let mut off = 0;
		let mut first_free_off = None;
		let mut free_length = 0;
		loop {
			let prev_off = off;
			let Some(ent) = next_dirent(self, superblock, io, buf, &mut off)? else {
				break;
			};
			// If the entry is on a new block, reset counter
			let new_block = off / blk_size > prev_off / blk_size;
			if ent.is_free() && !new_block {
				// Free entry, update counter
				first_free_off = first_free_off.or(Some(prev_off));
				free_length += ent.record_len();
				if free_length >= min_size.get() as usize {
					break;
				}
			} else {
				// Used entry, reset counter
				first_free_off = None;
				free_length = 0;
			}
		}
		Ok(first_free_off.map(|off| (off, free_length)))
	}

	/// Adds a new entry to the current directory.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	/// - `entry_inode` is the inode of the entry
	/// - `name` is the name of the entry
	/// - `file_type` is the type of the entry
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
		let mut buf = vec![0; blk_size as _]?;
		if let Some((off, len)) = self.get_suitable_slot(superblock, io, &mut buf, rec_len)? {
			let blk_off = (off / blk_size as u64) as u32;
			let inner_off = (off % buf.len() as u64) as usize;
			// Create used entry
			Dirent::write_new(
				&mut buf[inner_off..],
				superblock,
				entry_inode,
				rec_len.get(),
				file_type,
				name,
			)?;
			// Create free entries to cover remaining free space
			let mut i = inner_off + rec_len.get() as usize;
			let end = inner_off + len;
			while i < end {
				let rec_len = min(buf.len() - i, u16::MAX as usize) as u16;
				Dirent::write_new(&mut buf[i..], superblock, 0, rec_len, file_type, name)?;
				i += rec_len as usize;
			}
			// Write block
			let blk_off = self.translate_blk_off(blk_off, superblock, io)?.unwrap();
			write_block(blk_off.get() as _, superblock, io, &buf)?;
		} else {
			// No suitable free entry: Fill a new block
			let blk_off = (self.get_size(superblock) / blk_size as u64) as u32;
			let blk_off = self.alloc_content_blk(blk_off, superblock, io)?;
			buf.fill(0);
			// Create used entry
			Dirent::write_new(
				&mut buf,
				superblock,
				entry_inode,
				rec_len.get(),
				file_type,
				name,
			)?;
			// Create free entries to cover remaining free space
			let mut i = rec_len.get() as usize;
			while i < buf.len() {
				let rec_len = min(buf.len() - i, u16::MAX as usize) as u16;
				Dirent::write_new(&mut buf[i..], superblock, 0, rec_len, file_type, name)?;
				i += rec_len as usize;
			}
			// Write block
			write_block(blk_off.get() as _, superblock, io, &buf)?;
		}
		Ok(())
	}

	/// Removes the entry from the current directory.
	///
	/// Arguments:
	/// - `off` is the offset of the entry to remove
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	///
	/// If the entry does not exist, the function does nothing.
	///
	/// If the file is not a directory, the behaviour is undefined.
	pub fn remove_dirent(
		&mut self,
		off: u64,
		superblock: &mut Superblock,
		io: &mut dyn IO,
	) -> EResult<()> {
		debug_assert_eq!(self.get_type(), FileType::Directory);
		let blk_size = superblock.get_block_size() as u64;
		let file_blk_off = off / blk_size;
		let inner_off = (off % blk_size) as usize;
		// Read entry's block
		let mut buf = vec![0; blk_size as _]?;
		let Some(disk_blk_off) = self.translate_blk_off(file_blk_off as _, superblock, io)? else {
			return Ok(());
		};
		read_block(disk_blk_off.get() as _, superblock, io, &mut buf)?;
		// Read and free entry
		let ent = Dirent::from_slice(&mut buf[inner_off..], superblock)?;
		ent.inode = 0;
		// If the block is now empty, free it. Else, update it
		if is_block_empty(&mut buf, superblock)? {
			// If this is the last block, update the file's size
			let last_block = (file_blk_off + 1) * blk_size >= self.get_size(superblock);
			if last_block {
				let new_size = file_blk_off * blk_size;
				self.set_size(superblock, new_size);
			}
			self.free_content_blk(file_blk_off as _, superblock, io)
		} else {
			write_block(disk_blk_off.get() as _, superblock, io, &buf)
		}
	}

	/// Reads the content symbolic link.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	/// - `off` is the offset from which the link is read
	/// - `buf` is the buffer in which the content is written
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
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	/// - `buf` is the buffer in which the content is written
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

	/// Sets the device `major` and `minor`.
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
