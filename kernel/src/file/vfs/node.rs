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
		fs::{FileOps, Filesystem, NodeOps},
		FileType, INode, Stat,
	},
	memory::RcPage,
	sync::mutex::Mutex,
};
use core::ptr;
use utils::{
	boxed::Box,
	collections::{path::PathBuf, string::String, vec::Vec},
	errno::EResult,
	limits::SYMLINK_MAX,
	ptr::arc::Arc,
	vec,
};

/// A filesystem node, cached by the VFS.
#[derive(Debug)]
pub struct Node {
	/// Node ID
	pub inode: INode,
	/// The filesystem on which the node is located
	pub fs: Arc<Filesystem>,

	/// The node's status.
	pub stat: Mutex<Stat>,

	/// Handle for node operations
	pub node_ops: Box<dyn NodeOps>,
	/// Handle for open file operations
	pub file_ops: Box<dyn FileOps>,

	// TODO need a sparse array, inside of a rwlock
	/// Mapped pages
	pub pages: Mutex<Vec<Option<RcPage>>>,
}

impl Node {
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
		let mut buf = vec![0u8; INCREMENT]?;
		let mut len;
		loop {
			len = self.node_ops.readlink(self, &mut buf)?;
			if len < buf.len() || buf.len() >= SYMLINK_MAX {
				break;
			}
			buf.resize(buf.len() + INCREMENT, 0)?;
		}
		buf.truncate(len);
		PathBuf::try_from(String::from(buf))
	}

	/// Releases the node, removing it from the disk if this is the last reference to it.
	pub fn release(this: Arc<Self>) -> EResult<()> {
		{
			let mut cache = this.fs.node_cache.lock();
			// current instance + the one in `USED_NODE` = `2`
			if Arc::strong_count(&this) > 2 {
				return Ok(());
			}
			cache.remove(&this.inode);
		}
		// `unwrap` cannot fail since we removed it from the cache
		let node = Arc::into_inner(this).unwrap();
		node.try_remove()
	}

	/// Removes the node from the disk if it is orphan.
	pub fn try_remove(self) -> EResult<()> {
		// If there is no hard link left to the node, remove it
		let stat = self.stat.lock();
		let dir = stat.get_type() == Some(FileType::Directory);
		// If the file is a directory, the threshold is `1` because of the `.` entry
		let remove = (dir && stat.nlink <= 1) || stat.nlink == 0;
		if remove {
			self.fs.ops.destroy_node(&self)?;
		}
		Ok(())
	}
}
