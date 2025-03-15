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
		page_cache::PageCache,
		FileType, INode, Stat,
	},
	sync::mutex::Mutex,
};
use core::{
	borrow::Borrow,
	fmt,
	fmt::Formatter,
	hash::{Hash, Hasher},
	ptr,
};
use utils::{
	boxed::Box,
	collections::{hashmap::HashSet, path::PathBuf, string::String},
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

	/// A lock to be used by the filesystem implementation
	pub lock: Mutex<()>,
	/// The device's page cache
	pub cache: PageCache,
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
		// If other references are left, do nothing
		let Some(node) = Arc::into_inner(this) else {
			return Ok(());
		};
		// If there is no hard link left to the node, remove it
		let stat = node.stat.lock();
		let dir = stat.get_type() == Some(FileType::Directory);
		// If the file is a directory, the threshold is `1` because of the `.` entry
		let remove = (dir && stat.nlink <= 1) || stat.nlink == 0;
		if remove {
			node.fs.ops.destroy_node(&node)?;
		}
		Ok(())
	}
}

struct NodeWrapper(Arc<Node>);

impl Borrow<INode> for NodeWrapper {
	fn borrow(&self) -> &INode {
		&self.0.inode
	}
}

impl Eq for NodeWrapper {}

impl PartialEq for NodeWrapper {
	fn eq(&self, other: &Self) -> bool {
		self.0.inode == other.0.inode
	}
}

impl Hash for NodeWrapper {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.inode.hash(state)
	}
}

impl fmt::Debug for NodeWrapper {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.0, f)
	}
}

/// Cache for nodes for use inside filesystem implementations, to avoid duplications of [`Node`]
/// instances when several entries point to the same node.
#[derive(Debug, Default)]
pub struct NodeCache(Mutex<HashSet<NodeWrapper>>);

impl NodeCache {
	/// Inserts a node in cache. If already present, the previous entry is dropped.
	pub fn insert(&self, node: Arc<Node>) -> EResult<()> {
		self.0.lock().insert(NodeWrapper(node))?;
		Ok(())
	}

	/// Returns the node with ID `inode` from the cache, or if not in cache, initializes it with
	/// `init` and inserts it.
	pub fn get_or_insert<F: FnOnce() -> EResult<Arc<Node>>>(
		&self,
		inode: INode,
		init: F,
	) -> EResult<Arc<Node>> {
		let mut cache = self.0.lock();
		match cache.get(&inode) {
			// Cache hit
			Some(node) => Ok(node.0.clone()),
			// Cache miss, create instance and insert
			None => {
				let node = init()?;
				cache.insert(NodeWrapper(node.clone()))?;
				Ok(node)
			}
		}
	}

	/// If `node` is the last reference, removes it from the cache.
	pub fn release(&self, node: Arc<Node>) {
		// Lock now to avoid race condition
		let mut cache = self.0.lock();
		// this reference + the one in cache = 2
		if Arc::strong_count(&node) > 2 {
			return;
		}
		cache.remove(&node.inode);
	}
}
