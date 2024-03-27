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

use super::Superblock;
use crate::file::FileType;
use alloc::alloc::Global;
use core::{
	alloc::{Allocator, Layout},
	cmp::min,
	num::NonZeroU16,
	slice,
};
use utils::{
	boxed::Box,
	errno,
	errno::{AllocResult, EResult},
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

/// A directory entry is a structure stored in the content of an inode of type
/// `Directory`.
///
/// Each directory entry represent a file that is the stored in the
/// directory and points to its inode.
#[repr(C, packed)]
pub struct DirectoryEntry {
	/// The inode associated with the entry.
	pub(super) inode: u32,
	/// The total size of the entry.
	pub(super) total_size: u16,
	/// Name length least-significant bits.
	name_length_lo: u8,
	/// Name length most-significant bits or type indicator (if enabled).
	name_length_hi: u8,
	/// The entry's name.
	name: [u8],
}

impl DirectoryEntry {
	/// Creates a new free instance.
	///
	/// `total_size` is the size of the entry, including the name.
	pub fn new_free(total_size: NonZeroU16) -> AllocResult<Box<Self>> {
		let layout = Layout::from_size_align(total_size.get() as _, 8).unwrap();
		let slice = Global.allocate(layout)?;
		let mut entry = unsafe { Box::from_raw(slice.as_ptr() as *mut Self) };
		entry.total_size = total_size.get();
		Ok(entry)
	}

	/// Creates a new instance.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `inode` is the entry's inode.
	/// - `total_size` is the size of the entry, including the name.
	/// - `file_type` is the entry's type.
	/// - `name` is the entry's name.
	///
	/// If the given `inode` is zero, the entry is free.
	///
	/// If the total size is not large enough to hold the entry, the function
	/// returns an error.
	pub fn new(
		superblock: &Superblock,
		inode: u32,
		total_size: NonZeroU16,
		file_type: FileType,
		name: &[u8],
	) -> EResult<Box<Self>> {
		if (total_size.get() as usize) < (8 + name.len()) {
			return Err(errno!(EINVAL));
		}

		let mut entry = Self::new_free(total_size)?;
		entry.inode = inode;
		entry.set_type(superblock, file_type);
		entry.set_name(superblock, name);
		Ok(entry)
	}

	/// Creates a new instance from a slice.
	pub unsafe fn from(slice: &[u8]) -> AllocResult<Box<Self>> {
		let layout = Layout::from_size_align(slice.len(), 8).unwrap();
		let mut ptr = Global.allocate(layout)?.cast();
		let alloc_slice = slice::from_raw_parts_mut(ptr.as_mut(), slice.len());
		alloc_slice.copy_from_slice(slice);
		Ok(Box::from_raw(
			alloc_slice as *mut [u8] as *mut [()] as *mut Self,
		))
	}

	/// Returns the length the entry's name.
	///
	/// `superblock` is the filesystem's superblock.
	pub fn get_name_length(&self, superblock: &Superblock) -> usize {
		if superblock.required_features & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			((self.name_length_hi as usize) << 8) | (self.name_length_lo as usize)
		} else {
			self.name_length_lo as usize
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
	/// If the length of the entry is shorted than the required space, the name
	/// shall be truncated.
	pub fn set_name(&mut self, superblock: &Superblock, name: &[u8]) {
		let len = min(name.len(), self.total_size as usize - 8);
		self.name[..len].copy_from_slice(&name[..len]);

		self.name_length_lo = (len & 0xff) as u8;
		if superblock.required_features & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			self.name_length_hi = ((len >> 8) & 0xff) as u8;
		}
	}

	/// Returns the file type associated with the entry (if the option is
	/// enabled).
	pub fn get_type(&self, superblock: &Superblock) -> Option<FileType> {
		if superblock.required_features & super::REQUIRED_FEATURE_DIRECTORY_TYPE == 0 {
			match self.name_length_hi {
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
		}
	}

	/// Sets the file type associated with the entry (if the option is enabled).
	pub fn set_type(&mut self, superblock: &Superblock, file_type: FileType) {
		if superblock.required_features & super::REQUIRED_FEATURE_DIRECTORY_TYPE != 0 {
			self.name_length_hi = match file_type {
				FileType::Regular => TYPE_INDICATOR_REGULAR,
				FileType::Directory => TYPE_INDICATOR_DIRECTORY,
				FileType::CharDevice => TYPE_INDICATOR_CHAR_DEVICE,
				FileType::BlockDevice => TYPE_INDICATOR_BLOCK_DEVICE,
				FileType::Fifo => TYPE_INDICATOR_FIFO,
				FileType::Socket => TYPE_INDICATOR_SOCKET,
				FileType::Link => TYPE_INDICATOR_SYMLINK,
			};
		}
	}

	/// Tells whether the entry is valid.
	pub fn is_free(&self) -> bool {
		self.inode == 0
	}

	/// Tells whether the entry may be split to create a second entry with the
	/// given size `new_size`.
	pub fn may_split(&self, superblock: &Superblock, new_size: u16) -> bool {
		if self.is_free() {
			self.total_size > 16 + new_size
		} else {
			self.total_size - self.get_name_length(superblock) as u16 > 16 + new_size
		}
	}

	/// Splits the current entry into two entries and return the newly created
	/// entry.
	///
	/// `new_size` is the size of the new entry.
	pub fn split(&mut self, new_size: NonZeroU16) -> AllocResult<Box<Self>> {
		self.total_size -= new_size.get();
		DirectoryEntry::new_free(new_size)
	}

	/// Merges the current entry with the given entry `entry`.
	///
	/// If both entries are not on the same page or if `entry` is not located
	/// right after the current entry, the behaviour is undefined.
	pub fn merge(&mut self, entry: Box<Self>) {
		self.total_size += entry.total_size;
	}
}
