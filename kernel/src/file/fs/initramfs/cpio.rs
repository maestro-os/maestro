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

//! This module implements a CPIO format parser
//!
//! The kernel only support binary CPIO, not ASCII.

use crate::{file, file::FileType};
use core::mem::size_of;
use macros::AnyRepr;
use utils::bytes;

/// Entry type: FIFO
pub const TYPE_FIFO: u16 = 0x1000;
/// Entry type: Char device
pub const TYPE_CHAR_DEVICE: u16 = 0x2000;
/// Entry type: Directory
pub const TYPE_DIRECTORY: u16 = 0x4000;
/// Entry type: Block device
pub const TYPE_BLOCK_DEVICE: u16 = 0x6000;
/// Entry type: Regular file
pub const TYPE_REGULAR: u16 = 0x8000;
/// Entry type: Symbolic link
pub const TYPE_SYMLINK: u16 = 0xa000;
/// Entry type: Socket
pub const TYPE_SOCKET: u16 = 0xc000;

/// Rotates the given 4 bytes value from PDP-endian.
///
/// On PDP systems, long values (4 bytes) were stored as big endian, which means these values
/// need to be rotated to be read correctly.
pub fn rot_u32(v: u32) -> u32 {
	(v >> 16) | (v << 16)
}

/// Structure representing a CPIO header.
#[derive(AnyRepr, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct CPIOHeader {
	/// Magic value.
	pub c_magic: u16,
	/// Value uniquely identifying the entry.
	pub c_dev: u16,
	/// Value uniquely identifying the entry.
	pub c_ino: u16,
	/// The file's mode.
	pub c_mode: u16,
	/// The file owner's UID.
	pub c_uid: u16,
	/// The file owner's GID.
	pub c_gid: u16,
	/// The number of links referencing the file.
	pub c_nlink: u16,
	/// The implementation-defined details for character and block devices.
	pub c_rdev: u16,
	/// The timestamp of the latest time of modification of the file.
	pub c_mtime: u32,
	/// The length in bytes of the file's name.
	pub c_namesize: u16,
	/// The length in bytes of the file's content.
	pub c_filesize: u32,
}

impl CPIOHeader {
	/// Returns the file type associated with the entry.
	pub fn get_type(&self) -> FileType {
		let file_type = self.c_mode & 0xf000;

		match file_type {
			TYPE_FIFO => FileType::Fifo,
			TYPE_CHAR_DEVICE => FileType::CharDevice,
			TYPE_DIRECTORY => FileType::Directory,
			TYPE_BLOCK_DEVICE => FileType::BlockDevice,
			TYPE_REGULAR => FileType::Regular,
			TYPE_SYMLINK => FileType::Link,
			TYPE_SOCKET => FileType::Socket,
			_ => FileType::Regular,
		}
	}

	/// Returns the permissions of the entry.
	pub fn get_perms(&self) -> file::Mode {
		self.c_mode as file::Mode & 0x0fff
	}
}

/// A CPIO entry, consisting of a CPIO header, the filename and the content of the file.
pub struct CPIOEntry<'a> {
	/// The entry's data.
	data: &'a [u8],
}

impl<'a> CPIOEntry<'a> {
	/// Returns a reference to the header of the entry.
	pub fn get_hdr(&self) -> &'a CPIOHeader {
		// Will not fail because the structure is in range of the slice
		bytes::from_bytes::<CPIOHeader>(self.data).unwrap()
	}

	/// Returns a reference storing the filename.
	pub fn get_filename(&self) -> &'a [u8] {
		let hdr = self.get_hdr();

		let start = size_of::<CPIOHeader>();
		let mut end = start + hdr.c_namesize as usize;

		// Removing trailing NUL byte
		if end - start > 0 && self.data[end - 1] == b'\0' {
			end -= 1;
		}

		&self.data[start..end]
	}

	/// Returns a reference storing the content.
	pub fn get_content(&self) -> &'a [u8] {
		let hdr = self.get_hdr();

		let mut start = size_of::<CPIOHeader>() + hdr.c_namesize as usize;
		if start % 2 != 0 {
			start += 1;
		}

		let filesize = rot_u32(hdr.c_filesize);
		&self.data[start..(start + filesize as usize)]
	}
}

/// Structure representing a CPIO parser.
pub struct CPIOParser<'a> {
	/// The data to parse.
	data: &'a [u8],

	/// The current offset in data.
	curr_off: usize,
}

impl<'a> CPIOParser<'a> {
	/// Creates a new instance for the given data slice.
	pub fn new(data: &'a [u8]) -> Self {
		Self {
			data,

			curr_off: 0,
		}
	}
}

impl<'a> Iterator for CPIOParser<'a> {
	type Item = CPIOEntry<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		let off = self.curr_off;
		if off >= self.data.len() {
			return None;
		}

		let remaining_len = self.data.len() - off;
		if remaining_len < size_of::<CPIOHeader>() {
			return None;
		}

		// Will not fail because the structure is in range of the slice
		let hdr = bytes::from_bytes::<CPIOHeader>(&self.data[off..]).unwrap();

		// TODO: If invalid, check 0o707070. If valid, then data needs conversion (endianess)
		// Check magic
		if hdr.c_magic != 0o070707 {
			return None;
		}

		let mut namesize = hdr.c_namesize as usize;
		if namesize % 2 != 0 {
			namesize += 1;
		}

		let mut filesize = rot_u32(hdr.c_filesize) as usize;
		if filesize % 2 != 0 {
			filesize += 1;
		}

		let size = size_of::<CPIOHeader>() + namesize + filesize;
		if off + size > self.data.len() {
			return None;
		}

		self.curr_off += size;

		let entry = CPIOEntry {
			data: &self.data[off..(off + size)],
		};
		// Ignoring the entry if it is the last
		if entry.get_filename() == b"TRAILER!!!" {
			return None;
		}

		Some(entry)
	}
}
