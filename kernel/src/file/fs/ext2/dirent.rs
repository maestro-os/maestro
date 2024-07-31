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

use super::{Ext2INode, Superblock};
use crate::file::FileType;
use core::{cmp::min, mem::offset_of};
use macros::AnyRepr;
use utils::{errno, errno::EResult, io::IO};

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
	rec_len: u16,
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
		file_type: FileType,
		name: &[u8],
	) -> EResult<()> {
		// Validation
		if !slice.as_ptr().is_aligned_to(ALIGN) {
			return Err(errno!(EUCLEAN));
		}
		let name_len = name.len();
		if (rec_len as usize) > slice.len()
			|| (rec_len as usize) < NAME_OFF + name_len
			|| (rec_len as usize) % ALIGN != 0
		{
			return Err(errno!(EINVAL));
		}
		if name.len() > super::MAX_NAME_LEN {
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
		if !slice.as_ptr().is_aligned_to(ALIGN) {
			return Err(errno!(EUCLEAN));
		}
		if slice.len() < NAME_OFF {
			return Err(errno!(EUCLEAN));
		}
		// Read record's length
		const REC_LEN_OFF: usize = offset_of!(Dirent, rec_len);
		let rec_len = u16::from_le_bytes([slice[REC_LEN_OFF], slice[REC_LEN_OFF + 1]]) as usize;
		// Validation
		if rec_len > slice.len() || rec_len < NAME_OFF || rec_len % ALIGN != 0 {
			return Err(errno!(EUCLEAN));
		}
		// Reinterpret
		let ent = unsafe { &mut *(&mut slice[..rec_len] as *mut _ as *mut Self) };
		// Validation
		if !ent.is_free() && NAME_OFF + ent.name_len(superblock) > rec_len {
			return Err(errno!(EUCLEAN));
		}
		Ok(ent)
	}

	/// Returns the length of the record in bytes.
	///
	/// This value is never zero and always a multiple of [`ALIGN`].
	pub fn record_len(&self) -> usize {
		self.rec_len as _
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

	/// Sets the name of the entry.
	///
	/// If the length of the entry is shorter than the required space, the name
	/// shall be truncated.
	///
	/// If the name is too long, the function returns [`ENAMETOOLONG`].
	pub fn set_name(&mut self, superblock: &Superblock, name: &[u8]) -> EResult<()> {
		if name.len() > super::MAX_NAME_LEN {
			return Err(errno!(ENAMETOOLONG));
		}
		let len = min(name.len(), self.rec_len as usize - NAME_OFF);
		self.name[..len].copy_from_slice(&name[..len]);
		self.name_len = len as u8;
		// If the file type hint feature is not enabled, set the high byte of the name length to
		// zero
		if superblock.s_feature_incompat & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			self.file_type = 0;
		}
		Ok(())
	}

	/// Returns the file type associated with the entry.
	///
	/// If the type cannot be retrieved from the entry directly, the function retrieves it from the
	/// inode.
	pub fn get_type(&self, superblock: &Superblock, io: &mut dyn IO) -> EResult<FileType> {
		let ent_type =
			if superblock.s_feature_incompat & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
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
			} else {
				None
			};
		// If the type could not be retrieved from the entry itself, get it from the inode
		match ent_type {
			Some(t) => Ok(t),
			None => Ok(Ext2INode::read(self.inode, superblock, &mut *io)?.get_type()),
		}
	}

	/// Sets the file type associated with the entry (if the option is enabled).
	pub fn set_type(&mut self, superblock: &Superblock, file_type: FileType) {
		self.file_type =
			if superblock.s_feature_incompat & super::REQUIRED_FEATURE_DIRECTORY_TYPE != 0 {
				match file_type {
					FileType::Regular => TYPE_INDICATOR_REGULAR,
					FileType::Directory => TYPE_INDICATOR_DIRECTORY,
					FileType::CharDevice => TYPE_INDICATOR_CHAR_DEVICE,
					FileType::BlockDevice => TYPE_INDICATOR_BLOCK_DEVICE,
					FileType::Fifo => TYPE_INDICATOR_FIFO,
					FileType::Socket => TYPE_INDICATOR_SOCKET,
					FileType::Link => TYPE_INDICATOR_SYMLINK,
				}
			} else {
				// If the feature is not enabled, do nothing
				0
			};
	}

	/// Tells whether the entry is valid.
	pub fn is_free(&self) -> bool {
		self.inode == 0
	}
}
