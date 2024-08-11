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
	bgd::BlockGroupDescriptor, dirent, dirent::Dirent, read_block, write_block, Superblock,
};
use crate::{
	device::DeviceIO,
	file::{DirEntry, FileType, Mode},
};
use core::{
	cmp::{max, min},
	intrinsics::unlikely,
	mem,
	num::NonZeroU32,
};
use macros::AnyRepr;
use utils::{bytes, errno, errno::EResult, math, ptr::cow::Cow, vec};

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

/// The maximum length for a symlink to be stored in the inode itself instead of a
/// separate block.
const SYMLINK_INLINE_LIMIT: u64 = 60;

/// The inode of the root directory.
pub const ROOT_DIRECTORY_INODE: u32 = 2;
/// The root directory's default mode.
pub const ROOT_DIRECTORY_DEFAULT_MODE: u16 = INODE_PERMISSION_IRWXU
	| INODE_PERMISSION_IRGRP
	| INODE_PERMISSION_IXGRP
	| INODE_PERMISSION_IROTH
	| INODE_PERMISSION_IXOTH;

/// Computes the indirection offsets to reach the block at the linear offset `off`.
///
/// Arguments:
/// - `ent_per_blk_log` is the log2 of the number of entries in a block.
/// - `offsets` is the array to which the offsets are written.
///
/// On success, the function returns the number of offsets.
///
/// If the offset is out of bounds, the function returns [`EOVERFLOW`].
fn indirections_offsets(
	mut off: u32,
	ent_per_blk_log: u32,
	offsets: &mut [usize; 4],
) -> EResult<usize> {
	offsets.fill(0);
	if off < DIRECT_BLOCKS_COUNT as u32 {
		offsets[0] = off as _;
		return Ok(1);
	}
	off -= DIRECT_BLOCKS_COUNT as u32;
	let ent_per_blk = math::pow2(ent_per_blk_log as _);
	if off < ent_per_blk {
		offsets[0] = DIRECT_BLOCKS_COUNT;
		offsets[1] = off as _;
		return Ok(2);
	}
	off -= ent_per_blk;
	if off < ent_per_blk * ent_per_blk {
		offsets[0] = DIRECT_BLOCKS_COUNT + 1;
		offsets[1] = (off >> ent_per_blk_log) as _;
		offsets[2] = (off & (ent_per_blk - 1)) as _;
		return Ok(3);
	}
	off -= ent_per_blk * ent_per_blk;
	if off < ent_per_blk * ent_per_blk * ent_per_blk {
		offsets[0] = DIRECT_BLOCKS_COUNT + 2;
		offsets[1] = (off >> (ent_per_blk_log * 2)) as _;
		offsets[2] = ((off >> ent_per_blk_log) & (ent_per_blk_log - 1)) as _;
		offsets[3] = (off & (ent_per_blk - 1)) as _;
		return Ok(4);
	}
	Err(errno!(EOVERFLOW))
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

/// If no block is allocated at `blk`, allocate one.
///
/// On success, the function returns `blk`.
fn ensure_allocated(
	blk: &mut u32,
	superblock: &mut Superblock,
	io: &mut dyn DeviceIO,
) -> EResult<NonZeroU32> {
	if *blk == 0 {
		let new_blk = superblock.get_free_block(io)?;
		superblock.mark_block_used(io, new_blk)?;
		*blk = new_blk;
	}
	Ok(NonZeroU32::new(*blk).unwrap())
}

/// Returns the next directory entry.
///
/// Arguments:
/// - `node` is the node containing the entry
/// - `superblock` is the filesystem's superblock
/// - `io` is the I/O interface
/// - `buf` is the block buffer
/// - `off` is the offset of the entry to return
///
/// The [`Iterator`] trait cannot be used because of lifetime issues.
fn next_dirent<'b>(
	node: &Ext2INode,
	superblock: &Superblock,
	io: &mut dyn DeviceIO,
	buf: &'b mut [u8],
	off: u64,
) -> EResult<Option<&'b mut Dirent>> {
	let blk_size = superblock.get_block_size();
	let blk_off = (off / blk_size as u64) as u32;
	let inner_off = (off % blk_size as u64) as usize;
	// If at the beginning of a block, read it
	if inner_off == 0 {
		let res = node.translate_blk_off(blk_off, superblock, io);
		let blk_off = match res {
			Ok(Some(o)) => o,
			// If reaching a zero block, stop
			Ok(None) => return Ok(None),
			// If reaching the block limit, stop
			Err(e) if e.as_int() == errno::EOVERFLOW => return Ok(None),
			Err(e) => return Err(e),
		};
		read_block(blk_off.get() as _, blk_size, io, buf)?;
	}
	let ent = Dirent::from_slice(&mut buf[inner_off..], superblock)?;
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
		off += ent.rec_len as usize;
	}
	Ok(true)
}

/// An inode represents a file in the filesystem.
///
/// The name of the file is not included in the inode but in the directory entry associated with it
/// since several entries can refer to the same inode (hard links).
#[repr(C)]
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
	fn get_disk_offset(i: u32, superblock: &Superblock, io: &mut dyn DeviceIO) -> EResult<u64> {
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
	pub fn read(i: u32, superblock: &Superblock, io: &mut dyn DeviceIO) -> EResult<Self> {
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
	/// - `superblock` is the filesystem's superblock
	/// - `size` is the file's size
	/// - `inline` is `true` if the inode is a symlink storing the target inline
	fn set_size(&mut self, superblock: &Superblock, size: u64, inline: bool) {
		let has_version = superblock.s_rev_level >= 1;
		let has_feature = superblock.s_feature_ro_compat & super::WRITE_REQUIRED_64_BITS != 0;
		if has_version && has_feature {
			self.i_dir_acl = ((size >> 32) & 0xffffffff) as u32;
			self.i_size = (size & 0xffffffff) as u32;
		} else {
			self.i_size = size as u32;
		}
		if !inline {
			self.i_blocks = size.div_ceil(SECTOR_SIZE as _) as _;
		} else {
			self.i_blocks = 0;
		}
	}

	/// Returns the number of content blocks.
	pub fn get_blocks(&self) -> u32 {
		self.i_blocks / SECTOR_SIZE
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
		io: &mut dyn DeviceIO,
	) -> EResult<Option<NonZeroU32>> {
		let mut offsets: [usize; 4] = [0; 4];
		let depth =
			indirections_offsets(off, superblock.get_entries_per_block_log(), &mut offsets)?;
		let Some(mut blk) = check_blk_off(self.i_block[offsets[0]], superblock)? else {
			return Ok(None);
		};
		// Perform indirections
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0u8; blk_size as _]?;
		for off in &offsets[1..depth] {
			read_block(blk.get() as _, blk_size, io, &mut buf)?;
			let ents = bytes::slice_from_bytes(&buf);
			let Some(b) = check_blk_off(ents[*off], superblock)? else {
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
		io: &mut dyn DeviceIO,
	) -> EResult<NonZeroU32> {
		let mut offsets: [usize; 4] = [0; 4];
		let depth =
			indirections_offsets(off, superblock.get_entries_per_block_log(), &mut offsets)?;
		let mut blk = ensure_allocated(&mut self.i_block[offsets[0]], superblock, io)?;
		// Perform indirections
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0u8; blk_size as _]?;
		for off in &offsets[1..depth] {
			read_block(blk.get() as _, blk_size, io, &mut buf)?;
			let ents = bytes::slice_from_bytes_mut(&mut buf);
			let b = ensure_allocated(&mut ents[*off], superblock, io)?;
			// TODO avoided if unnecessary
			write_block(blk.get() as _, blk_size, io, &buf)?;
			blk = b;
		}
		Ok(blk)
	}

	fn free_content_blk_impl(
		blk: u32,
		offsets: &[usize],
		superblock: &mut Superblock,
		io: &mut dyn DeviceIO,
	) -> EResult<bool> {
		let Some(off) = offsets.first() else {
			return Ok(true);
		};
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0u8; blk_size as _]?;
		read_block(blk as _, blk_size, io, &mut buf)?;
		let ents = bytes::slice_from_bytes_mut(&mut buf);
		let b = &mut ents[*off];
		// Handle child block and determine whether the entry in the current block should be freed
		let free = Self::free_content_blk_impl(*b, &offsets[1..], superblock, io)?;
		if free {
			let b = mem::take(b);
			let empty = ents.iter().all(|b| *b == 0);
			if !empty {
				// The block is not empty, save
				write_block(blk as _, blk_size, io, &buf)?;
			}
			// If the block is empty, there is no point in saving it since it will be freed
			superblock.free_block(io, b)?;
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
		io: &mut dyn DeviceIO,
	) -> EResult<()> {
		let mut offsets: [usize; 4] = [0; 4];
		let depth =
			indirections_offsets(off, superblock.get_entries_per_block_log(), &mut offsets)?;
		let blk = &mut self.i_block[offsets[0]];
		if check_blk_off(*blk, superblock)?.is_none() {
			return Ok(());
		}
		if Self::free_content_blk_impl(*blk, &offsets[1..depth], superblock, io)? {
			let blk = mem::take(blk);
			superblock.free_block(io, blk)?;
		}
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
		io: &mut dyn DeviceIO,
	) -> EResult<u64> {
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
				read_block(blk_off.get() as _, blk_size, io, &mut blk_buff)?;
				let src = &blk_buff[blk_inner_off..(blk_inner_off + len as usize)];
				dst.copy_from_slice(src);
			} else {
				// No content block, writing zeros
				dst.fill(0);
			}
			cur += len;
		}
		Ok(cur)
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
		io: &mut dyn DeviceIO,
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
					read_block(blk_off.get() as _, blk_size, io, &mut blk_buff)?;
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
			write_block(blk_off.get() as _, blk_size, io, &blk_buff)?;
			cur += len;
		}
		// Update size
		let new_size = max(off + buff.len() as u64, curr_size);
		self.set_size(superblock, new_size, false);
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
		io: &mut dyn DeviceIO,
		size: u64,
	) -> EResult<()> {
		let old_size = self.get_size(superblock);
		if size >= old_size {
			return Ok(());
		}
		// Change the size
		self.set_size(superblock, size, false);
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
		blk: u32,
		level: usize,
		superblock: &mut Superblock,
		io: &mut dyn DeviceIO,
	) -> EResult<()> {
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0; blk_size as _]?;
		read_block(blk as _, blk_size, io, &mut buf)?;
		for blk in bytes::slice_from_bytes(&buf) {
			let Some(blk) = check_blk_off(*blk, superblock)? else {
				continue;
			};
			if let Some(next_level) = level.checked_sub(1) {
				Self::indirect_free_all(blk.get(), next_level, superblock, io)?;
			}
			superblock.free_block(io, blk.get())?;
		}
		Ok(())
	}

	/// Frees all the content blocks of the inode.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock
	/// - `io` is the I/O interface
	pub fn free_content(
		&mut self,
		superblock: &mut Superblock,
		io: &mut dyn DeviceIO,
	) -> EResult<()> {
		// If the file is a link and its content is stored inline, there is nothing to do
		if matches!(self.get_type(), FileType::Link)
			&& self.get_size(superblock) <= SYMLINK_INLINE_LIMIT
		{
			return Ok(());
		}
		self.set_size(superblock, 0, false);
		// TODO write inode
		// Free blocks
		for (off, blk) in self.i_block.iter().enumerate() {
			let Some(blk) = check_blk_off(*blk, superblock)? else {
				continue;
			};
			let depth = off.saturating_sub(DIRECT_BLOCKS_COUNT);
			if let Some(depth) = depth.checked_sub(1) {
				Self::indirect_free_all(blk.get(), depth, superblock, io)?;
			}
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
		io: &mut dyn DeviceIO,
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
		while let Some(ent) = next_dirent(self, superblock, io, &mut buf, off)? {
			if !ent.is_free() && ent.get_name(superblock) == name {
				return Ok(Some((ent.inode, ent.get_type(superblock, &mut *io)?, off)));
			}
			off += ent.rec_len as u64;
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
		mut off: u64,
		superblock: &Superblock,
		io: &mut dyn DeviceIO,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		if self.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// If the list is exhausted, stop
		if off >= self.get_size(superblock) {
			return Ok(None);
		}
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0; blk_size as _]?;
		// If the offset is not at the beginning of a block, read it so that `next_dirent` works
		// correctly
		if off % blk_size as u64 != 0 {
			let blk_off = off / blk_size as u64;
			let res = self.translate_blk_off(blk_off as _, superblock, io);
			let blk_off = match res {
				Ok(Some(o)) => o,
				// If reaching a zero block, stop
				Ok(None) => return Ok(None),
				// If reaching the block limit, stop
				Err(e) if e.as_int() == errno::EOVERFLOW => return Ok(None),
				Err(e) => return Err(e),
			};
			read_block(blk_off.get() as _, blk_size, io, &mut buf)?;
		}
		// Read the next entry, skipping free ones
		let ent = loop {
			// If no entry remain, stop
			let Some(ent) = next_dirent(self, superblock, io, &mut buf, off)? else {
				return Ok(None);
			};
			off += ent.rec_len as u64;
			if !ent.is_free() {
				break ent;
			}
		};
		let entry_type = ent.get_type(superblock, io)?;
		let name = ent.get_name(superblock).try_into()?;
		let ent = DirEntry {
			inode: ent.inode as _,
			entry_type,
			name: Cow::Owned(name),
		};
		Ok(Some((ent, off)))
	}

	/// Tells whether the current directory is empty.
	pub fn is_directory_empty(
		&self,
		superblock: &Superblock,
		io: &mut dyn DeviceIO,
	) -> EResult<bool> {
		let blk_size = superblock.get_block_size() as u64;
		let mut buf = vec![0; blk_size as _]?;
		let mut off = 0;
		while let Some(ent) = next_dirent(self, superblock, io, &mut buf, off)? {
			if !ent.is_free() {
				let name = ent.get_name(superblock);
				if name != b"." && name != b".." {
					return Ok(false);
				}
			}
			off += ent.rec_len as u64;
		}
		Ok(true)
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
		io: &mut dyn DeviceIO,
		buf: &mut [u8],
		min_size: u16,
	) -> EResult<Option<(u64, usize)>> {
		let blk_size = superblock.get_block_size() as u64;
		let mut off = 0;
		let mut free_length = 0;
		while let Some(ent) = next_dirent(self, superblock, io, buf, off)? {
			// If an entry is used but is able to fit the new entry, stop
			if !ent.is_free() && ent.can_fit(min_size, superblock) {
				return Ok(Some((off, ent.rec_len as _)));
			}
			// If the entry is used or on the next block
			let next = (off % blk_size + ent.rec_len as u64) > blk_size;
			if !ent.is_free() || next {
				// Reset counter
				free_length = 0;
			} else {
				// Free entry, update counter
				free_length += ent.rec_len as usize;
			}
			off += ent.rec_len as u64;
			// If a sequence large enough has been found, stop
			if free_length >= min_size as usize {
				let begin = off - free_length as u64;
				return Ok(Some((begin, free_length)));
			}
		}
		Ok(None)
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
		io: &mut dyn DeviceIO,
		entry_inode: u32,
		name: &[u8],
		file_type: FileType,
	) -> EResult<()> {
		debug_assert_eq!(self.get_type(), FileType::Directory);
		// If the name is too long, error
		if name.len() > super::MAX_NAME_LEN {
			return Err(errno!(ENAMETOOLONG));
		}
		let rec_len = (dirent::NAME_OFF + name.len()).next_multiple_of(dirent::ALIGN) as u16;
		// If the entry is too large, error
		let blk_size = superblock.get_block_size();
		if rec_len as u32 > blk_size {
			return Err(errno!(ENAMETOOLONG));
		}
		let mut buf = vec![0; blk_size as _]?;
		if let Some((mut off, mut len)) =
			self.get_suitable_slot(superblock, io, &mut buf, rec_len)?
		{
			// If the entry is used, shrink it
			let inner_off = (off % buf.len() as u64) as usize;
			let dirent = Dirent::from_slice(&mut buf[inner_off..], superblock)?;
			if !dirent.is_free() {
				let used_space = dirent.used_space(superblock);
				off += used_space as u64;
				len -= used_space as usize;
				dirent.rec_len = used_space;
			}
			// Create used entry
			let inner_off = (off % buf.len() as u64) as usize;
			Dirent::write_new(
				&mut buf[inner_off..],
				superblock,
				entry_inode,
				rec_len,
				Some(file_type),
				name,
			)?;
			// Create free entries to cover remaining free space
			let mut i = inner_off + rec_len as usize;
			let end = inner_off + len;
			while i < end {
				let rec_len = min(buf.len() - i, u16::MAX as usize) as u16;
				Dirent::write_new(&mut buf[i..], superblock, 0, rec_len, None, b"")?;
				i += rec_len as usize;
			}
			// Write block
			let blk_off = (off / blk_size as u64) as u32;
			let blk_off = self.translate_blk_off(blk_off, superblock, io)?.unwrap();
			write_block(blk_off.get() as _, blk_size, io, &buf)?;
		} else {
			// No suitable free entry: Fill a new block
			let blocks = self.get_blocks();
			let blk = self.alloc_content_blk(blocks, superblock, io)?;
			buf.fill(0);
			// Create used entry
			Dirent::write_new(
				&mut buf,
				superblock,
				entry_inode,
				rec_len,
				Some(file_type),
				name,
			)?;
			// Create free entries to cover remaining free space
			let mut i = rec_len as usize;
			while i < buf.len() {
				let rec_len = min(buf.len() - i, u16::MAX as usize) as u16;
				Dirent::write_new(&mut buf[i..], superblock, 0, rec_len, None, b"")?;
				i += rec_len as usize;
			}
			// Write block
			write_block(blk.get() as _, blk_size, io, &buf)?;
			self.set_size(superblock, (blocks as u64 + 1) * blk_size as u64, false);
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
		io: &mut dyn DeviceIO,
	) -> EResult<()> {
		debug_assert_eq!(self.get_type(), FileType::Directory);
		let blk_size = superblock.get_block_size();
		let file_blk_off = off / blk_size as u64;
		let inner_off = (off % blk_size as u64) as usize;
		// Read entry's block
		let mut buf = vec![0; blk_size as _]?;
		let Some(disk_blk_off) = self.translate_blk_off(file_blk_off as _, superblock, io)? else {
			return Ok(());
		};
		read_block(disk_blk_off.get() as _, blk_size, io, &mut buf)?;
		// Read and free entry
		let ent = Dirent::from_slice(&mut buf[inner_off..], superblock)?;
		ent.inode = 0;
		// If the block is now empty, free it. Else, update it
		if is_block_empty(&mut buf, superblock)? {
			// If this is the last block, update the file's size
			if file_blk_off as u32 + 1 >= self.get_blocks() {
				self.set_size(superblock, file_blk_off * blk_size as u64, false);
			}
			self.free_content_blk(file_blk_off as _, superblock, io)
		} else {
			write_block(disk_blk_off.get() as _, blk_size, io, &buf)
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
		io: &mut dyn DeviceIO,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		let size = self.get_size(superblock);
		if size <= SYMLINK_INLINE_LIMIT {
			// The target is stored inline in the inode
			let Some(len) = size.checked_sub(off) else {
				return Err(errno!(EINVAL));
			};
			// Copy
			let len = min(buf.len(), len as usize);
			let src = bytes::as_bytes(&self.i_block);
			let off = off as usize;
			buf[..len].copy_from_slice(&src[off..(off + len)]);
			Ok(len as _)
		} else {
			// The target is stored like in regular files
			self.read_content(off, buf, superblock, io)
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
		io: &mut dyn DeviceIO,
		buf: &[u8],
	) -> EResult<()> {
		let old_size = self.get_size(superblock);
		let new_size = buf.len() as u64;
		// Erase previous
		if old_size <= SYMLINK_INLINE_LIMIT {
			self.i_block.fill(0);
		}
		// Write target
		if new_size <= SYMLINK_INLINE_LIMIT {
			// The target is stored inline in the inode
			self.truncate(superblock, io, 0)?;
			// Copy
			let dst = bytes::as_bytes_mut(&mut self.i_block);
			dst[..buf.len()].copy_from_slice(buf);
			self.set_size(superblock, new_size, true);
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
	pub fn write(&self, i: u32, superblock: &Superblock, io: &mut dyn DeviceIO) -> EResult<()> {
		let off = Self::get_disk_offset(i, superblock, io)?;
		write(self, off, io)
	}
}
