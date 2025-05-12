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

//! A directory entry is an entry stored into an inode's content which
//! represents a subfile in a directory.

use super::{Ext2Fs, Superblock, read_block};
use crate::{
	file::{FileType, fs::ext2::inode::Ext2INode},
	memory::cache::RcFrame,
};
use core::{intrinsics::unlikely, mem::offset_of, ptr::NonNull};
use macros::AnyRepr;
use utils::{
	errno,
	errno::{EOVERFLOW, EResult},
	limits::NAME_MAX,
};

/// Directory entry type indicator: Unknown
const TYPE_INDICATOR_UNKNOWN: u8 = 0;
/// Directory entry type indicator: Regular file
const TYPE_INDICATOR_REGULAR: u8 = 1;
/// Directory entry type indicator: Directory
const TYPE_INDICATOR_DIRECTORY: u8 = 2;
/// Directory entry type indicator: Char device
const TYPE_INDICATOR_CHAR_DEVICE: u8 = 3;
/// Directory entry type indicator: Block device
const TYPE_INDICATOR_BLOCK_DEVICE: u8 = 4;
/// Directory entry type indicator: FIFO
const TYPE_INDICATOR_FIFO: u8 = 5;
/// Directory entry type indicator: Socket
const TYPE_INDICATOR_SOCKET: u8 = 6;
/// Directory entry type indicator: Symbolic link
const TYPE_INDICATOR_SYMLINK: u8 = 7;

/// The offset of the `name` field in [`Dirent`].
pub const NAME_OFF: usize = 8;
/// The alignment of directory entries.
pub const ALIGN: usize = 4;

/// A directory entry is a structure stored in the content of an inode of type
/// [`FileType::Directory`].
///
/// Each directory entry represent a file that is the stored in the
/// directory and points to its inode.
#[repr(C)]
#[derive(AnyRepr)]
pub struct Dirent {
	/// The inode associated with the entry.
	pub(super) inode: u32,
	/// The total size of the entry.
	pub(super) rec_len: u16,
	/// Name length least-significant bits.
	name_len: u8,
	/// Name length most-significant bits or type indicator (if enabled).
	file_type: u8,
	/// The entry's name.
	name: [u8],
}

impl Dirent {
	/// Writes a new entry onto the given slice of bytes.
	///
	/// Arguments:
	/// - `slice` is the slice to write the entry onto.
	/// - `superblock` is the filesystem's superblock
	/// - `entry_inode` is the target inode
	/// - `rec_len` is the length of the record
	/// - `file_type` is the file type hint of the entry
	/// - `name` is the name of the entry
	///
	/// If the parameters are invalid, the function return a corresponding error.
	pub fn write_new(
		slice: &mut [u8],
		superblock: &Superblock,
		entry_inode: u32,
		rec_len: u16,
		file_type: Option<FileType>,
		name: &[u8],
	) -> EResult<()> {
		// Validation
		let name_len = name.len();
		if unlikely(
			(rec_len as usize) > slice.len()
				|| (rec_len as usize) < NAME_OFF + name_len
				|| (rec_len as usize) % ALIGN != 0,
		) {
			return Err(errno!(EINVAL));
		}
		if unlikely(name.len() > NAME_MAX) {
			return Err(errno!(ENAMETOOLONG));
		}
		// Reinterpret
		let ent = unsafe { &mut *(&mut slice[..rec_len as usize] as *mut _ as *mut Self) };
		// Init
		ent.inode = entry_inode;
		ent.rec_len = rec_len;
		ent.set_type(superblock, file_type);
		ent.name[..name_len].copy_from_slice(name);
		ent.name_len = name_len as u8;
		Ok(())
	}

	/// Reinterprets a slice of bytes as a directory entry.
	///
	/// `superblock` is the filesystem's superblock.
	///
	/// If the entry is invalid, the function returns [`EUCLEAN`].
	pub fn from_slice<'b>(slice: &'b mut [u8], superblock: &Superblock) -> EResult<&'b mut Self> {
		// Validation
		if unlikely(slice.len() < NAME_OFF) {
			return Err(errno!(EUCLEAN));
		}
		// Read record's length
		const REC_LEN_OFF: usize = offset_of!(Dirent, rec_len);
		let rec_len = u16::from_le_bytes([slice[REC_LEN_OFF], slice[REC_LEN_OFF + 1]]) as usize;
		// Validation
		if unlikely(rec_len > slice.len() || rec_len < NAME_OFF || rec_len % ALIGN != 0) {
			return Err(errno!(EUCLEAN));
		}
		// Reinterpret
		let ent = unsafe { &mut *(&mut slice[..rec_len] as *mut _ as *mut Self) };
		// Validation
		if unlikely(!ent.is_free() && NAME_OFF + ent.name_len(superblock) > rec_len) {
			return Err(errno!(EUCLEAN));
		}
		Ok(ent)
	}

	/// Returns the length the entry's name.
	///
	/// `superblock` is the filesystem's superblock.
	pub fn name_len(&self, superblock: &Superblock) -> usize {
		if superblock.s_feature_incompat & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			((self.file_type as usize) << 8) | (self.name_len as usize)
		} else {
			self.name_len as usize
		}
	}

	/// Returns the entry's name.
	///
	/// `superblock` is the filesystem's superblock.
	pub fn get_name(&self, superblock: &Superblock) -> &[u8] {
		let name_length = self.name_len(superblock);
		&self.name[..name_length]
	}

	/// Returns the file type associated with the entry.
	///
	/// If the type cannot be retrieved from the entry directly, the function returns [`None`].
	pub fn get_type(&self, superblock: &Superblock) -> Option<FileType> {
		if superblock.s_feature_incompat & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			return None;
		}
		match self.file_type {
			TYPE_INDICATOR_REGULAR => Some(FileType::Regular),
			TYPE_INDICATOR_DIRECTORY => Some(FileType::Directory),
			TYPE_INDICATOR_CHAR_DEVICE => Some(FileType::CharDevice),
			TYPE_INDICATOR_BLOCK_DEVICE => Some(FileType::BlockDevice),
			TYPE_INDICATOR_FIFO => Some(FileType::Fifo),
			TYPE_INDICATOR_SOCKET => Some(FileType::Socket),
			TYPE_INDICATOR_SYMLINK => Some(FileType::Link),
			_ => None,
		}
	}

	/// Sets the file type associated with the entry (if the option is enabled).
	pub fn set_type(&mut self, superblock: &Superblock, file_type: Option<FileType>) {
		if superblock.s_feature_incompat & super::REQUIRED_FEATURE_DIRECTORY_TYPE != 0 {
			self.file_type = match file_type {
				None => TYPE_INDICATOR_UNKNOWN,
				Some(FileType::Regular) => TYPE_INDICATOR_REGULAR,
				Some(FileType::Directory) => TYPE_INDICATOR_DIRECTORY,
				Some(FileType::CharDevice) => TYPE_INDICATOR_CHAR_DEVICE,
				Some(FileType::BlockDevice) => TYPE_INDICATOR_BLOCK_DEVICE,
				Some(FileType::Fifo) => TYPE_INDICATOR_FIFO,
				Some(FileType::Socket) => TYPE_INDICATOR_SOCKET,
				Some(FileType::Link) => TYPE_INDICATOR_SYMLINK,
			};
		}
	}

	/// Tells whether the entry is valid.
	pub fn is_free(&self) -> bool {
		self.inode == 0
	}
}

/// An iterator over a directory's entries, including free ones.
///
/// The iterator returns the entry, along with its offset in the directory.
pub struct DirentIterator<'a> {
	/// The filesystem
	fs: &'a Ext2Fs,
	/// The directory's inode
	inode: &'a Ext2INode,

	/// The current block
	blk: &'a mut Option<RcFrame>,
	/// The current offset in the directory
	off: u64,
}

impl<'a> DirentIterator<'a> {
	/// Creates a new iterator over `inode`.
	///
	/// The iterator needs `blk` to be stored outside the iterator for lifetime reason.
	///
	/// `off` is the starting offset
	pub fn new(
		fs: &'a Ext2Fs,
		inode: &'a Ext2INode,
		blk: &'a mut Option<RcFrame>,
		off: u64,
	) -> EResult<Self> {
		*blk = Self::get_block(fs, inode, off)?;
		Ok(Self {
			fs,
			inode,

			blk,
			off,
		})
	}

	/// Reads the block for the entry at the offset `off`.
	///
	/// If reaching the end of the allocated blocks, the function returns `None`.
	fn get_block(fs: &Ext2Fs, inode: &Ext2INode, off: u64) -> EResult<Option<RcFrame>> {
		let blk_off = off / fs.sp.get_block_size() as u64;
		let res = inode.translate_blk_off(blk_off as _, fs);
		let blk_off = match res {
			Ok(Some(o)) => o,
			// If reaching a zero block, stop
			Ok(None) => return Ok(None),
			// If reaching the block limit, stop
			Err(e) if e.as_int() == EOVERFLOW => return Ok(None),
			Err(e) => return Err(e),
		};
		let blk = read_block(fs, blk_off.get() as _)?;
		Ok(Some(blk))
	}

	fn next_impl(&mut self) -> EResult<Option<(u64, &'a Dirent)>> {
		let blk_size = self.fs.sp.get_block_size() as u64;
		// If at the beginning of the block, read it
		let inner_off = (self.off % blk_size) as usize;
		if inner_off == 0 {
			*self.blk = Self::get_block(self.fs, self.inode, self.off)?;
		}
		// If no block remain, stop
		let Some(blk) = self.blk.as_mut() else {
			return Ok(None);
		};
		// Safe since the node is locked
		let blk_slice = unsafe { blk.slice_mut() };
		// Read entry
		let ent = Dirent::from_slice(&mut blk_slice[inner_off..], &self.fs.sp)?;
		let prev_off = self.off;
		self.off += ent.rec_len as u64;
		// If on the next block, ensure the offset is at the beginning
		if (prev_off / blk_size) != (self.off / blk_size) {
			self.off &= !(blk_size - 1);
		}
		// Use a `NonNull` to get the right lifetime
		let mut ent = NonNull::from(ent);
		Ok(Some((prev_off, unsafe { ent.as_mut() })))
	}
}

impl<'a> Iterator for DirentIterator<'a> {
	type Item = EResult<(u64, &'a Dirent)>;

	fn next(&mut self) -> Option<Self::Item> {
		self.next_impl().transpose()
	}
}
