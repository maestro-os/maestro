//! This module implements a CPIO format parser

use core::mem::size_of;
use crate::util;

/// Structure representing a CPIO header.
#[repr(C, packed)]
pub struct CPIOHeader {
	/// Magic value.
	c_magic: [u8; 6],
	/// Value uniquely identifying the entry.
	c_dev: [u8; 6],
	/// Value uniquely identifying the entry.
	c_ino: [u8; 6],
	/// The file's mode.
	c_mode: [u8; 6],
	/// The file owner's UID.
	c_uid: [u8; 6],
	/// The file owner's GID.
	c_gid: [u8; 6],
	/// The number of links referencing the file.
	c_nlink: [u8; 6],
	/// The implementation-defined details for character and block devices.
	c_rdev: [u8; 6],
	/// The timestamp of the latest time of modification of the file.
	c_mtime: [u8; 11],
	/// The length in bytes of the file's name.
	c_namesize: [u8; 6],
	/// The length in bytes of the file's content.
	c_filesize: [u8; 11],
}

/// A CPIO entry, consisting of a CPIO header, the filename and the content of the file.
pub struct CPIOEntry<'a> {
	/// The entry's data.
	data: &'a [u8],
}

impl<'a> CPIOEntry<'a> {
	// TODO
}

/// Structure representing a CPIO parser.
pub struct CPIOParser<'a> {
	/// The data to parse.
	data: &'a [u8],

	/// The current offset in data.
	curr_off: usize,
}

impl<'a> Iterator for CPIOParser<'a> {
	type Item = CPIOEntry<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.curr_off >= self.data.len() {
			return None;
		}

		let remaining_len = self.data.len() - self.curr_off;
		if remaining_len < size_of::<CPIOHeader>() {
			return None;
		}

		let _hdr = unsafe { // Safe because the structure is in range of the slice
			util::reinterpret::<CPIOHeader>(&self.data[self.curr_off])
		};

		// TODO
		todo!();
	}
}
