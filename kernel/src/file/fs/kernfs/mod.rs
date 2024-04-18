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

pub mod node;

use crate::{
	file::{
		fs::{Filesystem, NodeOps, Statfs},
		INode,
	},
	memory,
};
use core::cmp::{max, min};
use node::OwnedNode;
use utils::{boxed::Box, collections::vec::Vec, errno, errno::EResult, lock::Mutex, vec};

/// The index of the root inode.
pub const ROOT_INODE: INode = 1;

/// The maximum length of a name in the filesystem.
const MAX_NAME_LEN: usize = 255;

/// Storage of kernfs nodes.
///
/// Each element of the inner vector is a slot to store a node. If a slot is `None`, it means it is
/// free to be used.
#[derive(Debug)]
struct NodesStorage(Vec<Option<Box<dyn OwnedNode>>>);

impl NodesStorage {
	/// Returns an immutable reference to the node with inode `inode`.
	///
	/// If the node does not exist, the function returns an error.
	pub fn get_node(&self, inode: INode) -> EResult<&Box<dyn OwnedNode>> {
		self.0
			.get(inode as usize - 1)
			.ok_or_else(|| errno!(ENOENT))?
			.as_ref()
			.ok_or_else(|| errno!(ENOENT))
	}

	/// Returns a mutable reference to the node with inode `inode`.
	///
	/// If the node does not exist, the function returns an error.
	pub fn get_node_mut(&mut self, inode: INode) -> EResult<&mut Box<dyn OwnedNode>> {
		self.0
			.get_mut(inode as usize - 1)
			.ok_or_else(|| errno!(ENOENT))?
			.as_mut()
			.ok_or_else(|| errno!(ENOENT))
	}

	/// Returns mutable references to a pair of nodes.
	///
	/// If `a` == `b`, the function panics.
	///
	/// If at least one node does not exist, the function returns an error.
	pub fn get_node_pair_mut(
		&mut self,
		a: INode,
		b: INode,
	) -> EResult<(&mut Box<dyn OwnedNode>, &mut Box<dyn OwnedNode>)> {
		let a = a as usize - 1;
		let b = b as usize - 1;
		// Validation
		if a == b {
			panic!("kernfs: trying to get the same node twice at the same time");
		}
		if a >= self.0.len() || b >= self.0.len() {
			return Err(errno!(ENOENT));
		}
		// Split in two slices to allow taking two mutable references at once
		let min = min(a, b);
		let max = max(a, b);
		let (left, right) = self.0.split_at_mut(max);
		// Check if None and take references
		let left = left[min].as_mut().ok_or_else(|| errno!(ENOENT))?;
		let right = right[0].as_mut().ok_or_else(|| errno!(ENOENT))?;
		// Reorder according to arguments
		if a < b {
			Ok((left, right))
		} else {
			Ok((right, left))
		}
	}

	/// Returns a free slot for a new node.
	///
	/// If no slot is available, the function allocates a new one.
	pub fn get_free_slot(&mut self) -> EResult<(INode, &mut Option<Box<dyn OwnedNode>>)> {
		let slot = self
			.0
			.iter_mut()
			.enumerate()
			.find(|(_, s)| s.is_some())
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
	pub fn remove_node(&mut self, inode: INode) -> Option<Box<dyn OwnedNode>> {
		self.0
			.get_mut(inode as usize - 1)
			.map(Option::take)
			.flatten()
	}
}

/// A kernel file system.
///
/// `READ_ONLY` tells whether the filesystem is read-only.
#[derive(Debug)]
pub struct KernFS {
	/// Tells whether the filesystem is read-only.
	read_only: bool,
	/// Nodes storage.
	nodes: Mutex<NodesStorage>,
}

impl KernFS {
	/// Creates a new instance.
	///
	/// `root` is the root node of the filesystem.
	pub fn new(read_only: bool, root: Box<dyn OwnedNode>) -> EResult<Self> {
		Ok(Self {
			read_only,
			nodes: Mutex::new(NodesStorage(vec![Some(root)]?)),
		})
	}
}

impl Filesystem for KernFS {
	fn get_name(&self) -> &[u8] {
		b"kernfs"
	}

	fn is_readonly(&self) -> bool {
		self.read_only
	}

	fn use_cache(&self) -> bool {
		false
	}

	fn get_root_inode(&self) -> INode {
		ROOT_INODE
	}

	fn get_stat(&self) -> EResult<Statfs> {
		let nodes = self.nodes.lock();
		Ok(Statfs {
			f_type: 0,
			f_bsize: memory::PAGE_SIZE as _,
			f_blocks: 0,
			f_bfree: 0,
			f_bavail: 0,
			f_files: nodes.0.len() as _,
			f_ffree: 0,
			f_fsid: Default::default(),
			f_namelen: MAX_NAME_LEN as _,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn node_from_inode(&self, inode: INode) -> EResult<Box<dyn NodeOps>> {
		let nodes = self.nodes.lock();
		let node = nodes.get_node(inode)?;
		Ok(node.detached()?)
	}
}
