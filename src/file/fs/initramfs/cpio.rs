//! This module implements a CPIO format parser

use core::mem::size_of;
use core::ops::Add;
use core::ops::Mul;
use crate::util;

/// Converts the octal value stored in the given slice into an integer value.
/// If the value is invalid, the function returns None.
pub fn octal_to_integer<
	T: Add<Output = T>
		+ Clone
		+ Copy
		+ From<u8>
		+ Mul<Output = T>
>(val: &[u8]) -> Option<T> {
	let mut res = T::from(0u8);
	let mut pow = T::from(1u8);

	for c in val.iter().rev() {
		if *c < b'0' || *c > b'9' {
			return None;
		}

		let v = T::from((*c as u8) - b'0');
		res = res + (pow * v);

		pow = pow * T::from(8u8);
	}

	Some(res)
}

/// Structure representing a CPIO header.
#[repr(C, packed)]
pub struct CPIOHeader {
	/// Magic value.
	pub c_magic: [u8; 6],
	/// Value uniquely identifying the entry.
	pub c_dev: [u8; 6],
	/// Value uniquely identifying the entry.
	pub c_ino: [u8; 6],
	/// The file's mode.
	pub c_mode: [u8; 6],
	/// The file owner's UID.
	pub c_uid: [u8; 6],
	/// The file owner's GID.
	pub c_gid: [u8; 6],
	/// The number of links referencing the file.
	pub c_nlink: [u8; 6],
	/// The implementation-defined details for character and block devices.
	pub c_rdev: [u8; 6],
	/// The timestamp of the latest time of modification of the file.
	pub c_mtime: [u8; 11],
	/// The length in bytes of the file's name.
	pub c_namesize: [u8; 6],
	/// The length in bytes of the file's content.
	pub c_filesize: [u8; 11],
}

/// A CPIO entry, consisting of a CPIO header, the filename and the content of the file.
pub struct CPIOEntry<'a> {
	/// The entry's data.
	data: &'a [u8],
}

impl<'a> CPIOEntry<'a> {
	/// Returns a reference to the header of the entry.
	pub fn get_hdr(&self) -> &'a CPIOHeader {
		unsafe { // Safe because the structure is in range of the slice
			util::reinterpret::<CPIOHeader>(self.data)
		}
	}

	/// Returns a reference storing the filename.
	pub fn get_filename(&self) -> &'a [u8] {
		let hdr = self.get_hdr();
		// TODO Avoid unwrap
		let namesize = octal_to_integer::<usize>(&hdr.c_namesize).unwrap();

		let start = size_of::<CPIOHeader>();
		&self.data[start..(start + namesize)]
	}

	/// Returns a reference storing the content.
	pub fn get_content(&self) -> &'a [u8] {
		let hdr = self.get_hdr();
		// TODO Avoid unwrap
		let namesize = octal_to_integer::<usize>(&hdr.c_namesize).unwrap();
		let filesize = octal_to_integer::<usize>(&hdr.c_filesize).unwrap();

		let start = size_of::<CPIOHeader>() + namesize;
		&self.data[start..(start + filesize)]
	}
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

		let hdr = unsafe { // Safe because the structure is in range of the slice
			util::reinterpret::<CPIOHeader>(&self.data[self.curr_off..])
		};
		// TODO Avoid unwrap (check how to jump to the next record)
		let size = size_of::<CPIOHeader>()
			+ octal_to_integer::<usize>(&hdr.c_namesize).unwrap()
			+ octal_to_integer::<usize>(&hdr.c_filesize).unwrap();

		Some(CPIOEntry {
			data: &self.data[self.curr_off..(self.curr_off + size)],
		})
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn octal_to_integer0() {
		assert_eq!(octal_to_integer::<usize>(b"0").unwrap(), 0);
		assert_eq!(octal_to_integer::<usize>(b"00").unwrap(), 0);
		assert_eq!(octal_to_integer::<usize>(b"000").unwrap(), 0);
	}

	#[test_case]
	fn octal_to_integer1() {
		for i in b'0'..=b'9' {
			assert_eq!(octal_to_integer::<usize>(&[i]).unwrap(), (i - b'0') as usize);
		}
		for i in b'0'..=b'9' {
			assert_eq!(octal_to_integer::<usize>(&[b'0', i]).unwrap(), (i - b'0') as usize);
		}
		for i in b'0'..=b'9' {
			assert_eq!(octal_to_integer::<usize>(&[b'0', b'0', i]).unwrap(), (i - b'0') as usize);
		}
	}

	#[test_case]
	fn octal_to_integer2() {
		assert_eq!(octal_to_integer::<usize>(b"000000").unwrap(), 0);
		assert_eq!(octal_to_integer::<usize>(b"000001").unwrap(), 1);
		assert_eq!(octal_to_integer::<usize>(b"000002").unwrap(), 2);
		assert_eq!(octal_to_integer::<usize>(b"000003").unwrap(), 3);
		assert_eq!(octal_to_integer::<usize>(b"000004").unwrap(), 4);
		assert_eq!(octal_to_integer::<usize>(b"000005").unwrap(), 5);
		assert_eq!(octal_to_integer::<usize>(b"000006").unwrap(), 6);
		assert_eq!(octal_to_integer::<usize>(b"000007").unwrap(), 7);
		assert_eq!(octal_to_integer::<usize>(b"000010").unwrap(), 8);
		assert_eq!(octal_to_integer::<usize>(b"000011").unwrap(), 9);
		assert_eq!(octal_to_integer::<usize>(b"000012").unwrap(), 10);
		assert_eq!(octal_to_integer::<usize>(b"000013").unwrap(), 11);
		assert_eq!(octal_to_integer::<usize>(b"000014").unwrap(), 12);
		assert_eq!(octal_to_integer::<usize>(b"000015").unwrap(), 13);
		assert_eq!(octal_to_integer::<usize>(b"000016").unwrap(), 14);
		assert_eq!(octal_to_integer::<usize>(b"000017").unwrap(), 15);
		assert_eq!(octal_to_integer::<usize>(b"000020").unwrap(), 16);
		assert_eq!(octal_to_integer::<usize>(b"000021").unwrap(), 17);
		assert_eq!(octal_to_integer::<usize>(b"000022").unwrap(), 18);
		assert_eq!(octal_to_integer::<usize>(b"000023").unwrap(), 19);
		assert_eq!(octal_to_integer::<usize>(b"000024").unwrap(), 20);
		assert_eq!(octal_to_integer::<usize>(b"000025").unwrap(), 21);
		assert_eq!(octal_to_integer::<usize>(b"000026").unwrap(), 22);
		assert_eq!(octal_to_integer::<usize>(b"000027").unwrap(), 23);
		assert_eq!(octal_to_integer::<usize>(b"000030").unwrap(), 24);
	}

	#[test_case]
	fn octal_to_integer3() {
		assert!(octal_to_integer::<usize>(b"aaaaaa").is_none());
		assert!(octal_to_integer::<usize>(b"a01234").is_none());
		assert!(octal_to_integer::<usize>(b"00000a").is_none());
		assert!(octal_to_integer::<usize>(b"00a000").is_none());
		assert!(octal_to_integer::<usize>(b"a00000").is_none());
	}
}
