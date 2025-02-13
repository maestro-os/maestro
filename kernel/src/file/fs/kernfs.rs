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

//! A kernfs (kernel filesystem) is a virtual filesystem aiming at containing special files with
//! custom behaviours.
//!
//! This is often used to implement special filesystems that are used to ease communication between
//! the userspace and kernelspace.
//!
//! This module implements utilities for kernfs.

use crate::file::{
	fs::{FileOps, NodeOps},
	vfs,
	vfs::node::Node,
	DirEntry, File, FileType, INode, Stat,
};
use core::{
	cmp::min,
	fmt,
	fmt::{Debug, Write},
};
use utils::{
	boxed::Box,
	collections::vec::Vec,
	errno,
	errno::{AllocResult, EResult},
	ptr::{arc::Arc, cow::Cow},
	vec, DisplayableStr,
};

/// The index of the root inode.
pub const ROOT_INODE: INode = 1;

/// Storage of kernfs nodes.
///
/// Each element of the inner vector is a slot to store a node. If a slot is `None`, it means it is
/// free to be used.
#[derive(Debug)]
pub struct NodeStorage<N: NodeOps>(Vec<Option<N>>);

impl<N: NodeOps> NodeStorage<N> {
	/// Creates a new instance with the given root node.
	pub fn new(root: N) -> AllocResult<Self> {
		Ok(Self(vec![Some(root)]?))
	}

	/// Returns an immutable reference to the node with inode `inode`.
	///
	/// If the node does not exist, the function returns an error.
	pub fn get_node(&self, inode: INode) -> EResult<&N> {
		let index = (inode as usize)
			.checked_sub(1)
			.ok_or_else(|| errno!(ENOENT))?;
		self.0
			.get(index)
			.and_then(Option::as_ref)
			.ok_or_else(|| errno!(ENOENT))
	}

	/// Returns a free slot for a new node.
	///
	/// If no slot is available, the function allocates a new one.
	pub fn get_free_slot(&mut self) -> EResult<(INode, &mut Option<N>)> {
		let slot = self
			.0
			.iter_mut()
			.enumerate()
			.find(|(_, s)| s.is_none())
			.map(|(i, _)| i);
		let index = match slot {
			// Use an existing slot
			Some(i) => i,
			// Allocate a new node slot
			None => {
				let i = self.0.len();
				self.0.push(None)?;
				i
			}
		};
		let inode = index as u64 + 1;
		let slot = &mut self.0[index];
		Ok((inode, slot))
	}

	/// Removes the node with inode `inode`.
	///
	/// If the node is a non-empty directory, its content is **NOT** removed. It is the caller's
	/// responsibility to ensure no file is left allocated without a reference to it. Failure to do
	/// so results in a memory leak.
	///
	/// If the node doesn't exist, the function does nothing.
	pub fn remove_node(&mut self, inode: INode) -> Option<N> {
		self.0.get_mut(inode as usize - 1).and_then(Option::take)
	}
}

/// Writer for [`format_content_args`].
#[derive(Debug)]
struct FormatContentWriter<'b> {
	src_cursor: u64,
	dst: &'b mut [u8],
	dst_cursor: usize,
}

impl Write for FormatContentWriter<'_> {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		if s.is_empty() {
			return Ok(());
		}
		let chunk = s.as_bytes();
		// If at least part of the chunk is inside the range to read, copy
		if (chunk.len() as u64) > self.src_cursor {
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
		}
		// Update cursor
		self.src_cursor = self.src_cursor.saturating_sub(chunk.len() as _);
		Ok(())
	}
}

/// Implementation of [`crate::format_content`].
pub fn format_content_args(off: u64, buf: &mut [u8], args: fmt::Arguments<'_>) -> EResult<usize> {
	let mut writer = FormatContentWriter {
		src_cursor: off,
		dst: buf,
		dst_cursor: 0,
	};
	let res = fmt::write(&mut writer, args);
	if res.is_err() && (writer.dst_cursor < writer.dst.len()) {
		panic!("a formatting trait implementation returned an error");
	}
	Ok(writer.dst_cursor)
}

/// Formats the content of a kernfs node and write it on a buffer.
///
/// This is meant to be used in [`NodeOps::read_content`].
///
/// `off` and `buf` are the corresponding arguments from [`NodeOps::read_content`].
#[macro_export]
macro_rules! format_content {
    ($off:expr, $buf:expr, $($arg:tt)*) => {{
		$crate::file::fs::kernfs::format_content_args($off, $buf, format_args!($($arg)*))
	}};
}

/// A static symbolic link.
///
/// The inner value is the target of the symbolic link.
#[derive(Debug, Default)]
pub struct StaticLink(pub &'static [u8]);

impl FileOps for StaticLink {
	fn get_stat(&self, _file: &File) -> EResult<Stat> {
		Ok(Stat {
			mode: FileType::Link.to_mode() | 0o777,
			..Default::default()
		})
	}

	fn read(&self, _file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		format_content!(off, buf, "{}", DisplayableStr(self.0))
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
	pub init: fn(T) -> AllocResult<Box<dyn FileOps>>,
}

/// Helper to initialize an entry handle, using [`Default`].
pub fn entry_init_default<'e, E: 'e + FileOps + Default>(
	_: (),
) -> AllocResult<Box<dyn 'e + FileOps>> {
	box_wrap(E::default())
}

/// Helper to initialize an entry handle, using [`From`].
pub fn entry_init_from<'e, E: 'e + FileOps + From<T>, T>(
	val: T,
) -> AllocResult<Box<dyn 'e + FileOps>> {
	box_wrap(E::from(val))
}

/// Helper to wrap a [`FileOps`] into a [`Box`].
pub fn box_wrap<'n, N: 'n + FileOps>(ops: N) -> AllocResult<Box<dyn 'n + FileOps>> {
	Ok(Box::new(ops)? as _)
}

// TODO: the day Rust supports `dyn` in const generics (if it ever does), replace the entries array
// by a const generic
/// A read-only virtual directory used to point to other nodes.
#[derive(Debug)]
pub struct StaticDir<T: 'static + Clone + Debug = ()> {
	/// The directory's entries, sorted alphabetically by name.
	///
	/// **Warning**: If this array is not sorted correctly, the behaviour of
	/// [`NodeOps::lookup_entry`] is undefined.
	pub entries: &'static [StaticEntryBuilder<T>],
	/// Data used to initialize sub-nodes.
	pub data: T,
}

impl<T: 'static + Clone + Debug> StaticDir<T> {
	/// Inner implementation of [`Self::lookup_entry`].
	pub fn entry_by_name_inner(&self, ent: &mut vfs::Entry) -> EResult<()> {
		ent.node = self
			.entries
			.binary_search_by(|e| e.name.cmp(&ent.name))
			.ok()
			.map(|index| {
				let e = &self.entries[index];
				let ops = (e.init)(self.data.clone())?;
				Arc::new(Node {
					inode: 0,
					fs: Arc {},
					node_ops: (),
					file_ops: (),
					pages: Default::default(),
				})
			})
			.transpose()?;
		Ok(())
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
	fn get_stat(&self, _node: &Node) -> EResult<Stat> {
		Ok(Stat {
			mode: FileType::Directory.to_mode() | 0o555,
			..Default::default()
		})
	}

	fn lookup_entry(&self, _dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		self.entry_by_name_inner(ent)
	}

	fn next_entry(&self, _dir: &Node, off: u64) -> EResult<Option<(DirEntry<'static>, u64)>> {
		self.next_entry_inner(off)
	}
}

impl<T: 'static + Clone + Debug> FileOps for StaticDir<T> {}

#[cfg(test)]
mod test {
	#[test_case]
	fn content_chunks() {
		let val = 123;
		let mut out = [0u8; 9];
		let len = format_content!(0, &mut out, "{} {} {}", val, "def", "ghi").unwrap();
		assert_eq!(out.as_slice(), b"123 def g");
		assert_eq!(len, 9);
		let mut out = [0u8; 9];
		let len = format_content!(9, &mut out, "{} {} {}", "abc", "def", val).unwrap();
		assert_eq!(out.as_slice(), b"23\0\0\0\0\0\0\0");
		assert_eq!(len, 2);
		let mut out = [0u8; 9];
		let len = format_content!(3, &mut out, "{} {} {}", "abc", val, "ghi").unwrap();
		assert_eq!(out.as_slice(), b" 123 ghi\0");
		assert_eq!(len, 8);
		let mut out = [0u8; 9];
		let len = format_content!(4, &mut out, "{} {} {}", val, "def", "ghi").unwrap();
		assert_eq!(out.as_slice(), b"def ghi\0\0");
		assert_eq!(len, 7);
		let mut out = [0u8; 5];
		let len = format_content!(0, &mut out, "{} {} {}", "abc", val, "ghi").unwrap();
		assert_eq!(out.as_slice(), b"abc 1");
		assert_eq!(len, 5);
	}
}
