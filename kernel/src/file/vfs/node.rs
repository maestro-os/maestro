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

//! Filesystem node cache, allowing to handle hard links pointing to the same node.

use crate::{
	file::{
		FileType, INode, Stat,
		fs::{FileOps, Filesystem, NodeOps},
	},
	memory::{cache::MappedNode, user::UserSlice},
	sync::spin::Spin,
};
use core::ptr;
use utils::{
	boxed::Box,
	collections::{list::ListNode, path::PathBuf, string::String, vec::Vec},
	errno::EResult,
	limits::SYMLINK_MAX,
	ptr::arc::Arc,
};

/// A filesystem node, cached by the VFS.
#[derive(Debug)]
pub struct Node {
	/// Node ID
	pub inode: INode,
	/// The filesystem on which the node is located
	pub fs: Arc<Filesystem>,

	/// The node's status.
	///
	/// From the user of this structure's point of view, this is a read-only cache. It is updated
	/// only by the VFS
	pub stat: Spin<Stat>,

	/// Handle for node operations
	pub node_ops: Box<dyn NodeOps>,
	/// Handle for open file operations
	pub file_ops: Box<dyn FileOps>,

	/// A lock to be used by the filesystem implementation
	pub lock: Spin<()>,
	/// The node as mapped
	pub mapped: MappedNode,

	/// LRU node
	lru: ListNode,
}

impl Node {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `inode` is the node's ID
	/// - `fs` is the filesystem on which the node is located
	/// - `stat` is the node's status
	/// - `node_ops` is the handle for node operations
	/// - `file_ops` is the handle for open file operations
	pub fn new(
		inode: INode,
		fs: Arc<Filesystem>,
		stat: Stat,
		node_ops: Box<dyn NodeOps>,
		file_ops: Box<dyn FileOps>,
	) -> Self {
		Self {
			inode,
			fs,

			stat: Spin::new(stat),

			node_ops,
			file_ops,

			lock: Default::default(),
			mapped: Default::default(),

			lru: Default::default(),
		}
	}

	/// Returns the current status of the node.
	#[inline]
	pub fn stat(&self) -> Stat {
		self.stat.lock().clone()
	}

	/// Returns the type of the file.
	#[inline]
	pub fn get_type(&self) -> Option<FileType> {
		let stat = self.stat.lock();
		FileType::from_mode(stat.mode)
	}

	/// Tells whether the current node and `other` are on the same filesystem.
	#[inline]
	pub fn is_same_fs(&self, other: &Self) -> bool {
		ptr::eq(self.fs.as_ref(), other.fs.as_ref())
	}

	/// Reads the symbolic link.
	pub fn readlink(&self) -> EResult<PathBuf> {
		const INCREMENT: usize = 64;
		let mut buf = unsafe { Vec::new_uninit(INCREMENT)? };
		let mut len;
		loop {
			let b = UserSlice::from_slice_mut(&mut buf);
			len = self.node_ops.readlink(self, b)?;
			if len < buf.len() || buf.len() >= SYMLINK_MAX {
				break;
			}
			buf.resize(buf.len() + INCREMENT, 0)?;
		}
		buf.truncate(len);
		PathBuf::try_from(String::from(buf))
	}

	/// Synchronizes the node's cached content to disk.
	#[inline]
	pub fn sync_data(&self) -> EResult<()> {
		self.mapped.sync()
	}

	/// Releases the node, removing it from the disk if this is the last reference to it.
	pub fn release(this: Arc<Self>) -> EResult<()> {
		// If other references are left (aside from the one in the filesystem's cache), do nothing
		if Arc::strong_count(&this) > 2 {
			return Ok(());
		}
		let (file_type, nlink) = {
			let stat = this.stat.lock();
			(stat.get_type(), stat.nlink)
		};
		let dir = file_type == Some(FileType::Directory);
		// If there is no hard link left to the node, remove it
		// If the file is a directory, the threshold is `1` because of the `.` entry
		if (dir && nlink <= 1) || nlink == 0 {
			this.fs.ops.destroy_node(&this)?;
		}
		// Remove the node from the filesystem's cache
		this.fs.node_remove(this.inode);
		Ok(())
	}
}
