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

use crate::{
	file::{
		fs::{DummyOps, FileOps, NodeOps},
		vfs,
		vfs::node::Node,
		DirContext, DirEntry, FileType, INode, Stat,
	},
	memory::user::UserSlice,
	sync::mutex::Mutex,
};
use core::{
	cmp::min,
	fmt,
	fmt::{Debug, Write},
	sync::atomic::AtomicBool,
};
use utils::{
	boxed::Box,
	collections::vec::Vec,
	errno,
	errno::{AllocResult, EResult},
	ptr::arc::Arc,
	DisplayableStr,
};

/// The index of the root inode.
pub const ROOT_INODE: INode = 1;

/// Storage of kernfs nodes.
///
/// Each element of the inner vector is a slot to store a node. If a slot is `None`, it means it is
/// free to be used.
#[derive(Debug)]
pub struct NodeStorage(Vec<Option<Arc<Node>>>);

impl NodeStorage {
	/// Creates a new instance.
	pub fn new() -> AllocResult<Self> {
		Ok(Self(Vec::new()))
	}

	/// Returns an immutable reference to the node with inode `inode`.
	///
	/// If the node does not exist, the function returns an error.
	pub fn get_node(&self, inode: INode) -> EResult<&Arc<Node>> {
		let index = (inode as usize)
			.checked_sub(1)
			.ok_or_else(|| errno!(ENOENT))?;
		self.0
			.get(index)
			.and_then(Option::as_ref)
			.ok_or_else(|| errno!(ENOENT))
	}

	/// Sets the root node.
	pub fn set_root(&mut self, root: Arc<Node>) -> AllocResult<()> {
		if let Some(slot) = self.0.first_mut() {
			*slot = Some(root);
			Ok(())
		} else {
			self.0.push(Some(root))
		}
	}

	/// Returns a free slot for a new node.
	///
	/// If no slot is available, the function allocates a new one.
	pub fn get_free_slot(&mut self) -> EResult<(INode, &mut Option<Arc<Node>>)> {
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
	pub fn remove_node(&mut self, inode: INode) -> Option<Arc<Node>> {
		self.0.get_mut(inode as usize - 1).and_then(Option::take)
	}
}

/// Writer for [`format_content_args`].
#[derive(Debug)]
struct FormatContentWriter<'a> {
	src_cursor: u64,
	dst: UserSlice<'a, u8>,
	dst_cursor: usize,

	res: EResult<()>,
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
			// Write
			let res = self
				.dst
				.copy_to_user(self.dst_cursor, &chunk[src_cursor..(src_cursor + size)])
				.map(|_| ());
			self.res = self.res.and(res);
			if self.res.is_err() {
				return Err(fmt::Error);
			}
			self.dst_cursor += size;
		}
		// Update cursor
		self.src_cursor = self.src_cursor.saturating_sub(chunk.len() as _);
		Ok(())
	}
}

/// Implementation of [`crate::format_content`].
pub fn format_content_args(
	off: u64,
	buf: UserSlice<u8>,
	args: fmt::Arguments<'_>,
) -> EResult<usize> {
	let mut writer = FormatContentWriter {
		src_cursor: off,
		dst: buf,
		dst_cursor: 0,

		res: Ok(()),
	};
	let res = fmt::write(&mut writer, args);
	writer.res?;
	if res.is_err() && writer.dst_cursor < writer.dst.len() {
		panic!("a formatting trait implementation returned an error");
	}
	Ok(writer.dst_cursor)
}

/// Formats the content of a kernfs node and write it on a buffer.
///
/// This is meant to be used in [`FileOps::read`].
///
/// `off` and `buf` are the corresponding arguments from [`FileOps::read`].
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

impl NodeOps for StaticLink {
	fn readlink(&self, _node: &Node, buf: UserSlice<u8>) -> EResult<usize> {
		format_content!(0, buf, "{}", DisplayableStr(self.0))
	}
}

/// Either [`NodeOps`] or [`FileOps`] initializer.
#[derive(Debug)]
pub enum EitherOps<T> {
	/// Init [`NodeOps`]
	Node(fn(T) -> AllocResult<Box<dyn NodeOps>>),
	/// Init [`FileOps`]
	File(fn(T) -> AllocResult<Box<dyn FileOps>>),
}

/// An entry of a [`StaticDir`].
///
/// `T` is the type of the parameter passed to `init`.
#[derive(Debug)]
pub struct StaticEntry<T = ()> {
	/// The name of the entry
	pub name: &'static [u8],
	/// The node's status
	pub stat: fn(T) -> Stat,
	/// A builder which returns a handle to perform operations
	pub init: EitherOps<T>,
}

/// Helper to wrap a [`FileOps`] into a [`Box`].
pub fn box_file<'n, N: 'n + FileOps>(ops: N) -> AllocResult<Box<dyn 'n + FileOps>> {
	Ok(Box::new(ops)? as _)
}

/// Helper to wrap a [`NodeOps`] into a [`Box`].
pub fn box_node<'n, N: 'n + NodeOps>(ops: N) -> AllocResult<Box<dyn 'n + NodeOps>> {
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
	pub entries: &'static [StaticEntry<T>],
	/// Data used to initialize sub-nodes.
	pub data: T,
}

/// Returns [`Stat`] for [`StaticDir`].
#[inline]
pub fn static_dir_stat() -> Stat {
	Stat {
		mode: FileType::Directory.to_mode() | 0o555,
		..Default::default()
	}
}

impl<T: 'static + Clone + Debug> NodeOps for StaticDir<T> {
	fn lookup_entry(&self, dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		ent.node = self
			.entries
			.binary_search_by(|e| e.name.cmp(&ent.name))
			.ok()
			.map(|index| {
				let ent = &self.entries[index];
				let stat = (ent.stat)(self.data.clone());
				let (node_ops, file_ops) = match ent.init {
					EitherOps::Node(init) => {
						let node_ops = init(self.data.clone())?;
						let file_ops = Box::new(DummyOps)? as _;
						(node_ops, file_ops)
					}
					EitherOps::File(init) => {
						let node_ops = Box::new(DummyOps)? as _;
						let file_ops = init(self.data.clone())?;
						(node_ops, file_ops)
					}
				};
				Arc::new(Node {
					inode: 0,
					fs: dir.fs.clone(),

					stat: Mutex::new(stat),
					dirty: AtomicBool::new(false),

					node_ops,
					file_ops,

					lock: Default::default(),
					mapped: Default::default(),
				})
			})
			.transpose()?;
		Ok(())
	}

	fn iter_entries(&self, _dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let iter = self.entries.iter().skip(ctx.off as usize);
		for e in iter {
			let stat = (e.stat)(self.data.clone());
			let ent = DirEntry {
				inode: 0,
				entry_type: stat.get_type(),
				name: e.name,
			};
			if !(ctx.write)(&ent)? {
				break;
			}
			ctx.off += 1;
		}
		Ok(())
	}
}

#[cfg(test)]
mod test {
	use crate::memory::user::UserSlice;

	#[test_case]
	fn content_chunks() {
		let val = 123;
		let mut out = [0u8; 9];
		let len = format_content!(
			0,
			UserSlice::from_slice_mut(&mut out),
			"{} {} {}",
			val,
			"def",
			"ghi"
		)
		.unwrap();
		assert_eq!(out.as_slice(), b"123 def g");
		assert_eq!(len, 9);

		let mut out = [0u8; 9];
		let len = format_content!(
			9,
			UserSlice::from_slice_mut(&mut out),
			"{} {} {}",
			"abc",
			"def",
			val
		)
		.unwrap();
		assert_eq!(out.as_slice(), b"23\0\0\0\0\0\0\0");
		assert_eq!(len, 2);

		let mut out = [0u8; 9];
		let len = format_content!(
			3,
			UserSlice::from_slice_mut(&mut out),
			"{} {} {}",
			"abc",
			val,
			"ghi"
		)
		.unwrap();
		assert_eq!(out.as_slice(), b" 123 ghi\0");
		assert_eq!(len, 8);

		let mut out = [0u8; 9];
		let len = format_content!(
			4,
			UserSlice::from_slice_mut(&mut out),
			"{} {} {}",
			val,
			"def",
			"ghi"
		)
		.unwrap();
		assert_eq!(out.as_slice(), b"def ghi\0\0");
		assert_eq!(len, 7);

		let mut out = [0u8; 5];
		let len = format_content!(
			0,
			UserSlice::from_slice_mut(&mut out),
			"{} {} {}",
			"abc",
			val,
			"ghi"
		)
		.unwrap();
		assert_eq!(out.as_slice(), b"abc 1");
		assert_eq!(len, 5);
	}
}
