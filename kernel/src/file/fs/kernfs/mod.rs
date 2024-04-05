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
		fs::{kernfs::node::DefaultNode, Filesystem, Statfs},
		DirEntry, File, FileLocation, FileType, INode,
	},
	memory,
};
use core::cmp::{max, min};
use node::KernFSNode;
use utils::{boxed::Box, collections::vec::Vec, errno, errno::EResult, ptr::cow::Cow, vec};

/// The index of the root inode.
pub const ROOT_INODE: INode = 1;

/// The maximum length of a name in the filesystem.
const MAX_NAME_LEN: usize = 255;

/// If the `node` is a directory, the function inserts entries `.` and `..` if not present,
/// updating the number of hard links at the same time. The function also increments the hard links
/// count on `parent` if necessary.
///
/// If `parent` is `None`, `node` is considered being its own parent.
///
/// On failure, `node` is left altered midway, but not `parent`.
fn insert_base_entries(
	node: &mut dyn KernFSNode,
	parent: Option<&mut dyn KernFSNode>,
) -> EResult<()> {
	if node.get_file_type() != FileType::Directory {
		return Ok(());
	}
	let mut new_links = 0;
	if node.entry_by_name(b".")?.is_none() {
		node.add_entry(DirEntry {
			inode: ROOT_INODE,
			entry_type: FileType::Directory,
			name: Cow::Borrowed(b"."),
		})?;
		new_links += 1;
	}
	if node.entry_by_name(b"..")?.is_none() {
		node.add_entry(DirEntry {
			inode: ROOT_INODE,
			entry_type: FileType::Directory,
			name: Cow::Borrowed(b".."),
		})?;
		if let Some(parent) = parent {
			parent.set_hard_links_count(parent.get_hard_links_count() + 1);
		} else {
			new_links += 1;
		}
	}
	node.set_hard_links_count(node.get_hard_links_count() + new_links);
	Ok(())
}

/// Returns the file for the given `node` and `inode`.
fn load_file_impl(inode: INode, node: &dyn KernFSNode) -> File {
	let mut file = File::new(
		node.get_uid(),
		node.get_gid(),
		node.get_file_type(),
		node.get_mode(),
	);
	file.location = FileLocation::Filesystem {
		mountpoint_id: 0, // dummy value to be replaced
		inode,
	};
	file.set_hard_links_count(node.get_hard_links_count());
	file.set_size(node.get_size());
	file.ctime = node.get_ctime();
	file.mtime = node.get_mtime();
	file.atime = node.get_atime();
	file
}

/// A kernel file system.
///
/// `READ_ONLY` tells whether the filesystem is read-only.
#[derive(Debug)]
pub struct KernFS<const READ_ONLY: bool> {
	/// The list of nodes of the filesystem.
	///
	/// The index in this vector is the inode.
	nodes: Vec<Option<Box<dyn KernFSNode>>>,
}

impl<const READ_ONLY: bool> KernFS<READ_ONLY> {
	/// Creates a new instance.
	pub fn new() -> EResult<Self> {
		Ok(Self {
			nodes: vec![None]?,
		})
	}

	/// Sets the root node of the filesystem.
	pub fn set_root(&mut self, mut root: Box<dyn KernFSNode>) -> EResult<()> {
		insert_base_entries(root.as_mut(), None)?;
		// Insert
		if self.nodes.is_empty() {
			self.nodes.push(Some(root))?;
		} else {
			self.nodes[ROOT_INODE as usize - 1] = Some(root);
		}
		Ok(())
	}

	/// Returns an immutable reference to the node with inode `inode`.
	///
	/// If the node does not exist, the function returns an error.
	pub fn get_node(&self, inode: INode) -> EResult<&Box<dyn KernFSNode>> {
		self.nodes
			.get(inode as usize - 1)
			.ok_or_else(|| errno!(ENOENT))?
			.as_ref()
			.ok_or_else(|| errno!(ENOENT))
	}

	/// Returns a mutable reference to the node with inode `inode`.
	///
	/// If the node does not exist, the function returns an error.
	pub fn get_node_mut(&mut self, inode: INode) -> EResult<&mut Box<dyn KernFSNode>> {
		self.nodes
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
	) -> EResult<(&mut Box<dyn KernFSNode>, &mut Box<dyn KernFSNode>)> {
		let a = a as usize - 1;
		let b = b as usize - 1;
		// Validation
		if a == b {
			panic!("kernfs: trying to get the same node twice at the same time");
		}
		if a >= self.nodes.len() || b >= self.nodes.len() {
			return Err(errno!(ENOENT));
		}
		// Split in two slices to allow taking two mutable references at once
		let min = min(a, b);
		let max = max(a, b);
		let (left, right) = self.nodes.split_at_mut(max);
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

	/// Adds the given node `node` to the filesystem.
	///
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, node: Box<dyn KernFSNode>) -> EResult<INode> {
		let slot = self.nodes.iter_mut().enumerate().find(|(_, s)| s.is_some());
		let inode = if let Some((inode, slot)) = slot {
			// Use an existing slot
			*slot = Some(node);
			inode
		} else {
			// Allocate a new node slot
			let inode = self.nodes.len();
			self.nodes.push(Some(node))?;
			inode
		};
		Ok((inode + 1) as _)
	}

	/// Removes the node with inode `inode`.
	///
	/// If the node is a non-empty directory, its content is **NOT** removed. It is the caller's
	/// responsibility to ensure no file is left allocated without a reference to it. Failure to do
	/// so results in a memory leak.
	///
	/// If the node doesn't exist, the function does nothing.
	pub fn remove_node(&mut self, inode: INode) -> Option<Box<dyn KernFSNode>> {
		self.nodes
			.get_mut(inode as usize - 1)
			.map(Option::take)
			.flatten()
	}

	/// Adds a file to the kernfs.
	///
	/// Arguments
	/// - `parent_inode` is the inode of the parent directory in which the file is inserted.
	/// - `node` is the node of the new file.
	/// - `name` is the name of the new file.
	///
	/// On success, the function returns the inode of the newly inserted file.
	fn add_file_impl<N: 'static + KernFSNode>(
		&mut self,
		parent_inode: INode,
		node: N,
		name: &[u8],
	) -> EResult<INode> {
		// Insert node and get a reference to it along with its parent
		let inode = self.add_node(Box::new(node)?)?;
		let (parent, node) = match self.get_node_pair_mut(parent_inode, inode) {
			Ok(n) => n,
			// On error, rollback the previous insertion
			Err(e) => {
				self.remove_node(inode);
				return Err(e);
			}
		};
		// Add base entries
		if let Err(e) = insert_base_entries(node.as_mut(), Some(parent.as_mut())) {
			// Rollback node insertion
			self.remove_node(inode);
			return Err(e);
		}
		// Add entry to parent
		let res = parent.add_entry(DirEntry {
			inode,
			entry_type: node.get_file_type(),
			name: Cow::Borrowed(name),
		});
		if let Err(e) = res {
			// Rollback hard link from the `..` entry
			if node.get_file_type() == FileType::Directory {
				let cnt = node.get_hard_links_count().saturating_sub(1);
				node.set_hard_links_count(cnt);
			}
			// Rollback node insertion
			self.remove_node(inode);
			return Err(e);
		}
		Ok(inode)
	}
}

impl<const READ_ONLY: bool> Filesystem for KernFS<READ_ONLY> {
	fn get_name(&self) -> &[u8] {
		&[]
	}

	fn is_readonly(&self) -> bool {
		READ_ONLY
	}

	fn use_cache(&self) -> bool {
		false
	}

	fn get_root_inode(&self) -> INode {
		ROOT_INODE
	}

	fn get_stat(&self) -> EResult<Statfs> {
		Ok(Statfs {
			f_type: 0,
			f_bsize: memory::PAGE_SIZE as _,
			f_blocks: 0,
			f_bfree: 0,
			f_bavail: 0,
			f_files: self.nodes.len() as _,
			f_ffree: 0,
			f_fsid: Default::default(),
			f_namelen: MAX_NAME_LEN as _,
			f_frsize: 0,
			f_flags: 0,
		})
	}

	fn load_file(&self, inode: INode) -> EResult<File> {
		let node = self.get_node(inode)?;
		Ok(load_file_impl(inode, node.as_ref()))
	}

	fn add_file(&self, parent_inode: INode, name: &[u8], file: File) -> EResult<File> {
		if READ_ONLY {
			return Err(errno!(EROFS));
		}
		let mut node = DefaultNode::new(file.uid, file.gid, file.file_type, file.mode);
		node.set_atime(file.atime);
		node.set_ctime(file.ctime);
		node.set_mtime(file.mtime);
		let inode = self.add_file_impl(parent_inode, node, name)?;
		// Cannot fail as the node was just inserted
		let node = self.get_node(inode).unwrap();
		Ok(load_file_impl(inode, node.as_ref()))
	}

	fn add_link(&self, parent_inode: INode, name: &[u8], inode: INode) -> EResult<()> {
		if READ_ONLY {
			return Err(errno!(EROFS));
		}
		let (parent, node) = self.get_node_pair_mut(parent_inode, inode)?;
		if parent.get_file_type() == FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Insert the new entry
		parent.add_entry(DirEntry {
			inode,
			entry_type: node.get_file_type(),
			name: Cow::Borrowed(name),
		})?;
		node.set_hard_links_count(node.get_hard_links_count() + 1);
		Ok(())
	}

	fn update_inode(&self, file: &File) -> EResult<()> {
		if READ_ONLY {
			return Err(errno!(EROFS));
		}
		let node = self.get_node_mut(file.location.get_inode())?;
		node.set_uid(file.get_uid());
		node.set_gid(file.get_gid());
		node.set_mode(file.get_mode());
		node.set_ctime(file.ctime);
		node.set_mtime(file.mtime);
		node.set_atime(file.atime);
		Ok(())
	}

	fn remove_file(&self, parent_inode: INode, name: &[u8]) -> EResult<(u16, INode)> {
		if READ_ONLY {
			return Err(errno!(EROFS));
		}
		// Get directory
		let parent = self.get_node_mut(parent_inode)?;
		if parent.get_file_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Get node to remove
		let (inode, entry_off) = parent.entry_by_name(name)?.ok_or_else(|| errno!(ENOENT))?;
		let inode = inode.inode;
		let (parent, node) = self.get_node_pair_mut(parent_inode, inode)?;
		// If the node is a non-empty directory, error
		if !node.is_directory_empty()? {
			return Err(errno!(ENOTEMPTY));
		}
		// If no link is left, remove the node
		let links = node.get_hard_links_count().saturating_sub(1);
		node.set_hard_links_count(links);
		if node.get_hard_links_count() == 0 {
			self.remove_node(inode);
		}
		// Remove directory entry
		parent.remove_entry(entry_off);
		// If the node is a directory, decrement the number of hard links to the parent
		// (because of the entry `..` in the removed node)
		if node.get_file_type() == FileType::Directory {
			parent.set_hard_links_count(parent.get_hard_links_count().saturating_sub(1));
		}
		Ok((links, inode))
	}
}
