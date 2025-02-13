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
		FileType, INode,
	},
	memory::buddy::PageState,
	sync::mutex::Mutex,
};
use utils::{boxed::Box, collections::vec::Vec, errno::EResult, ptr::arc::Arc};

/// A filesystem node, cached by the VFS.
#[derive(Debug)]
pub struct Node {
	/// Node ID
	pub inode: INode,
	/// The filesystem on which the node is located
	pub fs: Arc<Filesystem>,

	/// Handle for node operations
	pub node_ops: Box<dyn NodeOps>,
	/// Handle for open file operations
	pub file_ops: Box<dyn FileOps>,

	// TODO need a sparse array, inside of a rwlock
	/// Mapped pages
	pub pages: Mutex<Vec<&'static PageState>>,
}

impl Node {
	/// Releases the node, removing it from the disk if this is the last reference to it.
	pub fn release(this: Arc<Self>) -> EResult<()> {
		// Lock to avoid race condition later
		let mut used_nodes = USED_NODES.lock();
		// current instance + the one in `USED_NODE` = `2`
		if Arc::strong_count(&this) > 2 {
			return Ok(());
		}
		used_nodes.remove(&this.location);
		let Some(node) = Arc::into_inner(this) else {
			return Ok(());
		};
		node.try_remove()
	}

	/// Removes the node from the disk if it is orphan.
	pub fn try_remove(self) -> EResult<()> {
		// If there is no hard link left to the node, remove it
		let stat = self.node_ops.get_stat(&self)?;
		let dir = stat.get_type() == Some(FileType::Directory);
		// If the file is a directory, the threshold is `1` because of the `.` entry
		let remove = (dir && stat.nlink <= 1) || stat.nlink == 0;
		if remove {
			self.fs.superblock.destroy_node(&self)?;
		}
		Ok(())
	}
}
