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

//! KernFS node utilities.

use crate::file::{
	fs::{Filesystem, NodeOps},
	DirEntry, FileType, INode, Stat,
};
use core::{
	cmp::min,
	fmt,
	fmt::{Debug, Write},
};
use utils::{
	boxed::Box,
	errno,
	errno::{AllocResult, EResult},
	ptr::cow::Cow,
	DisplayableStr,
};

/// A node that is owned by the kernfs, or another entity (such as another node).
///
/// An `OwnedNode` *may* or *may not* have an associated inode.
pub trait OwnedNode: NodeOps {
	/// Returns the operations handle for the node to be used outside the kernfs.
	///
	/// This handle can carry additional information, but it is recommended to use a ZST if
	/// possible so that no memory allocation has to be made.
	fn detached(&self) -> AllocResult<Box<dyn NodeOps>>;
}

/// Writer for [`format_content_args`].
#[derive(Debug)]
struct FormatContentWriter<'b> {
	src_cursor: u64,
	dst: &'b mut [u8],
	dst_cursor: usize,
	eof: bool,
}

impl<'b> Write for FormatContentWriter<'b> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		if s.is_empty() {
			return Ok(());
		}
		let chunk = s.as_bytes();
		// If at least part of the chunk is inside the range to read, copy
		if (chunk.len() as u64) > self.src_cursor {
			self.eof = false;
			// If the end of the output buffer is reached, stop
			if self.dst_cursor >= self.dst.len() {
				return Err(fmt::Error);
			}
			// Offset and size of the range in the chunk to copy
			let src_cursor = self.src_cursor as usize;
			let size = min(
				self.dst.len().saturating_sub(self.dst_cursor),
				chunk.len().saturating_sub(src_cursor),
			);
			self.dst[self.dst_cursor..(self.dst_cursor + size)]
				.copy_from_slice(&chunk[src_cursor..(src_cursor + size)]);
			self.dst_cursor += size;
			// If the end of the chunk is reached, set `eof` if necessary
			if src_cursor + size >= chunk.len() {
				// If other non-empty chunks remain, the next iteration will cancel this
				self.eof = true;
			}
		}
		// Update cursor
		self.src_cursor = self.src_cursor.saturating_sub(chunk.len() as _);
		Ok(())
	}
}

/// Implementation of [`crate::format_content`].
pub fn format_content_args(
	off: u64,
	buf: &mut [u8],
	args: fmt::Arguments<'_>,
) -> EResult<(u64, bool)> {
	let mut writer = FormatContentWriter {
		src_cursor: off,
		dst: buf,
		dst_cursor: 0,
		eof: true,
	};
	let res = fmt::write(&mut writer, args);
	if res.is_err() && (writer.dst_cursor < writer.dst.len()) {
		panic!("a formatting trait implementation returned an error");
	}
	Ok((writer.dst_cursor as _, writer.eof))
}

/// Formats the content of a kernfs node and write it on a buffer.
///
/// This is meant to be used in [`NodeOps::read_content`].
///
/// `off` and `buf` are the corresponding arguments from [`NodeOps::read_content`].
#[macro_export]
macro_rules! format_content {
    ($off:expr, $buf:expr, $($arg:tt)*) => {{
		$crate::file::fs::kernfs::node::format_content_args($off, $buf, format_args!($($arg)*))
	}};
}

/// A static symbolic link pointing to a constant target.
#[derive(Debug, Default)]
pub struct StaticLink<const TARGET: &'static [u8]>;

impl<const TARGET: &'static [u8]> NodeOps for StaticLink<TARGET> {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		Ok(Stat {
			file_type: FileType::Link,
			mode: 0o777,
			..Default::default()
		})
	}

	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<(u64, bool)> {
		format_content!(off, buf, "{}", DisplayableStr(TARGET))
	}
}

/// A builder for an entry of a [`StaticDir`].
///
/// `T` is the type of the parameter passed to `init`.
#[derive(Debug)]
pub struct StaticEntryBuilder<T = ()> {
	/// The name of the entry.
	pub name: &'static [u8],
	/// The type of the entry.
	pub entry_type: FileType,
	/// A builder which returns a handle to perform operations on the node.
	pub init: fn(T) -> AllocResult<Box<dyn NodeOps>>,
}

/// Helper to initialize an entry handle, using [`Default`].
pub fn entry_init_default<'e, E: 'e + NodeOps + Default>(
	_: (),
) -> AllocResult<Box<dyn 'e + NodeOps>> {
	box_wrap(E::default())
}

/// Helper to initialize an entry handle, using [`From`].
pub fn entry_init_from<'e, E: 'e + NodeOps + From<T>, T>(
	val: T,
) -> AllocResult<Box<dyn 'e + NodeOps>> {
	box_wrap(E::from(val))
}

/// Helper to wrap a [`NodeOps`] into a [`Box`].
pub fn box_wrap<'n, N: 'n + NodeOps>(ops: N) -> AllocResult<Box<dyn 'n + NodeOps>> {
	Ok(Box::new(ops)? as _)
}

// TODO: the day Rust supports `dyn` in const generics (if it ever does), replace the entries array
// by a const generic
/// A read-only virtual directory used to point to other nodes.
#[derive(Debug)]
pub struct StaticDir<T: 'static + Clone + Debug = ()> {
	/// The directory's entries, sorted alphabeticaly by name.
	///
	/// **Warning**: If this array is not sorted correctly, the behaviour of
	/// [`NodeOps::entry_by_name`] is undefined.
	pub entries: &'static [StaticEntryBuilder<T>],
	/// Data used to initialize sub-nodes.
	pub data: T,
}

impl<T: 'static + Clone + Debug> StaticDir<T> {
	/// Inner implementation of [`Self::entry_by_name`].
	pub fn entry_by_name_inner<'n>(
		&self,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		let Ok(index) = self.entries.binary_search_by(|e| e.name.cmp(name)) else {
			return Ok(None);
		};
		let e = &self.entries[index];
		let ops = (e.init)(self.data.clone())?;
		Ok(Some((
			DirEntry {
				inode: 0,
				entry_type: e.entry_type,
				name: Cow::Borrowed(name),
			},
			ops,
		)))
	}

	/// Inner implementation of [`Self::next_entry`].
	pub fn next_entry_inner(&self, off: u64) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let off: usize = off.try_into().map_err(|_| errno!(EINVAL))?;
		let Some(e) = self.entries.get(off) else {
			return Ok(None);
		};
		Ok(Some((
			DirEntry {
				inode: 0,
				entry_type: e.entry_type,
				name: Cow::Borrowed(e.name),
			},
			(off + 1) as _,
		)))
	}
}

impl<T: 'static + Clone + Debug> NodeOps for StaticDir<T> {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		Ok(Stat {
			file_type: FileType::Directory,
			mode: 0o555,
			..Default::default()
		})
	}

	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		self.entry_by_name_inner(name)
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		self.next_entry_inner(off)
	}
}

#[cfg(test)]
mod test {
	#[test_case]
	fn content_chunks() {
		let val = 123;
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(0, &mut out, "{} {} {}", val, "def", "ghi").unwrap();
		assert_eq!(out.as_slice(), b"123 def g");
		assert_eq!(len, 9);
		assert!(!eof);
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(9, &mut out, "{} {} {}", "abc", "def", val).unwrap();
		assert_eq!(out.as_slice(), b"23\0\0\0\0\0\0\0");
		assert_eq!(len, 2);
		assert!(eof);
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(3, &mut out, "{} {} {}", "abc", val, "ghi").unwrap();
		assert_eq!(out.as_slice(), b" 123 ghi\0");
		assert_eq!(len, 8);
		assert!(eof);
		let mut out = [0u8; 9];
		let (len, eof) = format_content!(4, &mut out, "{} {} {}", val, "def", "ghi").unwrap();
		assert_eq!(out.as_slice(), b"def ghi\0\0");
		assert_eq!(len, 7);
		assert!(eof);
		let mut out = [0u8; 5];
		let (len, eof) = format_content!(0, &mut out, "{} {} {}", "abc", val, "ghi").unwrap();
		assert_eq!(out.as_slice(), b"abc 1");
		assert_eq!(len, 5);
		assert!(!eof);
	}
}
