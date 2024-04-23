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
use alloc::alloc::Global;
use core::{
	alloc::{Allocator, Layout},
	cmp::min,
	mem::offset_of,
	num::NonZeroU16,
};
use macros::AnyRepr;
use utils::{
	boxed::Box,
	errno,
	errno::{AllocResult, EResult},
	io::IO,
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

/// The offset of the `name` field in [`DirectoryEntry`].
const NAME_OFF: usize = 8;

/// A directory entry is a structure stored in the content of an inode of type
/// `Directory`.
///
/// Each directory entry represent a file that is the stored in the
/// directory and points to its inode.
#[repr(C, packed)]
#[derive(AnyRepr)]
pub struct DirectoryEntry {
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

impl DirectoryEntry {
	/// Creates a new free instance.
	///
	/// `rec_len` is the size of the entry, including the name.
	pub fn new_free(rec_len: NonZeroU16) -> AllocResult<Box<Self>> {
		let layout = Layout::from_size_align(rec_len.get() as _, 8).unwrap();
		let slice = Global.allocate(layout)?;
		let mut entry = unsafe { Box::from_raw(slice.as_ptr() as *mut Self) };
		entry.rec_len = rec_len.get();
		Ok(entry)
	}

	/// Creates a new instance.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `inode` is the entry's inode.
	/// - `rec_len` is the size of the entry, including the name.
	/// - `file_type` is the entry's type.
	/// - `name` is the entry's name.
	///
	/// If the given `inode` is zero, the entry is free.
	///
	/// If the total size is not large enough to hold the entry, the function
	/// returns an error.
	///
	/// If the name is too long, the function returns [`ENAMETOOLONG`].
	pub fn new(
		superblock: &Superblock,
		inode: u32,
		rec_len: NonZeroU16,
		file_type: FileType,
		name: &[u8],
	) -> EResult<Box<Self>> {
		// Validation
		if (rec_len.get() as usize) < (NAME_OFF + name.len()) {
			return Err(errno!(EINVAL));
		}
		let mut entry = Self::new_free(rec_len)?;
		entry.inode = inode;
		entry.set_type(superblock, file_type);
		entry.set_name(superblock, name)?;
		Ok(entry)
	}

	/// Reinterprets a slice of bytes as a directory entry.
	///
	/// `superblock` is the filesystem's superblock.
	///
	/// If the entry is invalid, the function returns [`EUCLEAN`].
	pub fn from(slice: &[u8], superblock: &Superblock) -> EResult<Box<Self>> {
		// Read record's length
		const REC_LEN_OFF: usize = offset_of!(DirectoryEntry, rec_len);
		if slice.len() < NAME_OFF {
			return Err(errno!(EUCLEAN));
		}
		let rec_len = u16::from_le_bytes([slice[REC_LEN_OFF], slice[REC_LEN_OFF + 1]]) as usize;
		// Bound check
		if rec_len < NAME_OFF || rec_len > slice.len() {
			return Err(errno!(EUCLEAN));
		}
		// Reinterpret
		let ent = unsafe { &*(&slice[..rec_len] as *const _ as *const Self) };
		// Validation
		if !ent.is_free() && NAME_OFF + ent.get_name_length(superblock) > rec_len {
			return Err(errno!(EUCLEAN));
		}
		// Allocate and copy
		let layout = Layout::from_size_align(rec_len, 8).unwrap();
		let mut ptr = Global.allocate(layout)?;
		unsafe {
			ptr.as_mut().copy_from_slice(&slice[..rec_len]);
			Ok(Box::from_raw(ptr.as_ptr() as *mut Self))
		}
	}

	/// Returns the length the entry's name.
	///
	/// `superblock` is the filesystem's superblock.
	pub fn get_name_length(&self, superblock: &Superblock) -> usize {
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
		let name_length = self.get_name_length(superblock);
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
		// If the feature is not enabled, do nothing
		if superblock.s_feature_incompat & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			return;
		}
		self.file_type = match file_type {
			FileType::Regular => TYPE_INDICATOR_REGULAR,
			FileType::Directory => TYPE_INDICATOR_DIRECTORY,
			FileType::CharDevice => TYPE_INDICATOR_CHAR_DEVICE,
			FileType::BlockDevice => TYPE_INDICATOR_BLOCK_DEVICE,
			FileType::Fifo => TYPE_INDICATOR_FIFO,
			FileType::Socket => TYPE_INDICATOR_SOCKET,
			FileType::Link => TYPE_INDICATOR_SYMLINK,
		};
	}

	/// Tells whether the entry is valid.
	pub fn is_free(&self) -> bool {
		self.inode == 0
	}

	/// Tells whether the current entry (free or not) may be suitable to fit a new used entry with
	/// the given size `size`.
	pub fn would_fit(&self, superblock: &Superblock, size: NonZeroU16) -> bool {
		let available = if self.is_free() {
			self.rec_len
		} else {
			self.rec_len - self.get_name_length(superblock) as u16
		};
		available >= size.get()
	}

	/// Splits the current entry into two entries and return the newly created
	/// entry.
	///
	/// `size` is the size of the new entry.
	pub fn insert(&mut self, size: NonZeroU16) -> AllocResult<Box<Self>> {
		if self.is_free() {
			// If the entry is free, use it as a whole
			// `rec_len` is never zero
			let size: NonZeroU16 = self.rec_len.try_into().unwrap();
			DirectoryEntry::new_free(size)
		} else {
			// If the entry is used, split it to use the unused space
			self.rec_len -= size.get();
			DirectoryEntry::new_free(size)
		}
	}

	/// Merges the current entry with the given entry `entry`.
	///
	/// If both entries are not on the same page or if `entry` is not located
	/// right after the current entry, the behaviour is undefined.
	pub fn merge(&mut self, entry: Box<Self>) {
		self.rec_len += entry.rec_len;
	}
}
