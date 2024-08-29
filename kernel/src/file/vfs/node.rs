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

use crate::file::{fs::NodeOps, FileLocation, FileType};
use core::{
	borrow::Borrow,
	hash::{Hash, Hasher},
};
use utils::{
	boxed::Box,
	collections::hashmap::HashSet,
	errno::{AllocResult, EResult},
	lock::Mutex,
	ptr::arc::Arc,
};

/// A filesystem node, cached by the VFS.
#[derive(Debug)]
pub struct Node {
	/// The location of the file on a filesystem.
	pub location: FileLocation,
	/// Handle for node operations.
	pub ops: Box<dyn NodeOps>,
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
		// The reference count is now `1`
		let Some(node) = Arc::into_inner(this) else {
			unreachable!();
		};
		// If there is no hard link left to the node, remove it
		let stat = node.ops.get_stat(&node.location)?;
		let dir = stat.get_type() == Some(FileType::Directory);
		// If the file is a directory, the threshold is `1` because of the `.` entry
		let remove = (dir && stat.nlink <= 1) || stat.nlink == 0;
		if remove {
			node.ops.remove_file(&node.location)?;
		}
		Ok(())
	}
}

/// An entry in the nodes cache.
///
/// The [`Hash`] and [`PartialEq`] traits are forwarded to the entry's location.
#[derive(Debug)]
struct NodeEntry(Arc<Node>);

impl Borrow<FileLocation> for NodeEntry {
	fn borrow(&self) -> &FileLocation {
		&self.0.location
	}
}

impl Eq for NodeEntry {}

impl PartialEq for NodeEntry {
	fn eq(&self, other: &Self) -> bool {
		self.0.location.eq(&other.0.location)
	}
}

impl Hash for NodeEntry {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.location.hash(state)
	}
}

/// The list of nodes current in use.
static USED_NODES: Mutex<HashSet<NodeEntry>> = Mutex::new(HashSet::new());

/// Looks in the nodes cache for the node with the given location. If not in cache, the node is
/// created and inserted.
pub(super) fn get_or_insert(location: FileLocation, ops: Box<dyn NodeOps>) -> EResult<Arc<Node>> {
	let mut used_nodes = USED_NODES.lock();
	let node = used_nodes.get(&location).map(|e| e.0.clone());
	match node {
		Some(node) => Ok(node),
		// The node is not in cache. Insert it
		None => {
			// Create and insert node
			let node = Arc::new(Node {
				location,
				ops,
			})?;
			used_nodes.insert(NodeEntry(node.clone()))?;
			Ok(node)
		}
	}
}

/// Inserts a new node in cache.
pub(super) fn insert(node: Node) -> AllocResult<Arc<Node>> {
	let mut used_nodes = USED_NODES.lock();
	let node = Arc::new(node)?;
	used_nodes.insert(NodeEntry(node.clone()))?;
	Ok(node)
}
