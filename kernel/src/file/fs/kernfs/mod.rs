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

//! Kernfs implements utilities allowing to create a virtual filesystem.

pub mod content;
pub mod node;

use crate::{
	file::{
		fs::{kernfs::node::DummyKernFSNode, Filesystem, Statfs},
		perm::{Gid, Uid},
		DirEntry, File, FileContent, FileLocation, FileType, INode, Mode,
	},
	memory,
	process::oom,
};
use core::{alloc::AllocError, borrow::Borrow, intrinsics::unlikely};
use node::KernFSNode;
use utils::{
	boxed::Box,
	collections::{string::String, vec::Vec},
	errno,
	errno::EResult,
	io::IO,
	vec, TryClone,
};

// TODO Change to `1`
/// The index of the root inode.
pub const ROOT_INODE: INode = 0;

/// The maximum length of a name in the filesystem.
const MAX_NAME_LEN: usize = 255;

/// A kernel file system.
#[derive(Debug)]
pub struct KernFS {
	/// The name of the filesystem.
	name: String,
	/// Tells whether the filesystem is readonly.
	readonly: bool,

	/// The list of nodes of the filesystem.
	///
	/// The index in this vector is the inode.
	nodes: Vec<Option<Box<dyn KernFSNode>>>,
	/// A list of free inodes.
	free_nodes: Vec<INode>,
}

impl KernFS {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `name` is the name of the filesystem.
	/// - `readonly` tells whether the filesystem is readonly.
	pub fn new(name: String, readonly: bool) -> EResult<Self> {
		Ok(Self {
			name,
			readonly,

			nodes: vec![None]?,
			free_nodes: Vec::new(),
		})
	}

	/// Sets the root node of the filesystem.
	pub fn set_root(&mut self, mut root: Box<dyn KernFSNode>) -> EResult<()> {
		// Adding `.` and `..` entries if the new file is a directory
		let mut content = root.get_content()?;
		let mut new_links = 0;
		if let FileContent::Directory(ref mut entries) = &mut *content {
			if !entries.contains_key(b".".as_slice()) {
				entries.insert(
					b".".as_slice().try_into()?,
					DirEntry {
						inode: ROOT_INODE,
						entry_type: FileType::Directory,
					},
				)?;
				new_links += 1;
			}
			if !entries.contains_key(b"..".as_slice()) {
				entries.insert(
					b"..".as_slice().try_into()?,
					DirEntry {
						inode: ROOT_INODE,
						entry_type: FileType::Directory,
					},
				)?;
				new_links += 1;
			}
		}
		let new_cnt = root.get_hard_links_count() + new_links;
		root.set_hard_links_count(new_cnt);

		if self.nodes.is_empty() {
			self.nodes.push(Some(root))?;
		} else {
			self.nodes[ROOT_INODE as usize] = Some(root);
		}

		Ok(())
	}

	/// Returns an immutable reference to the node with inode `inode`.
	///
	/// If the node doesn't exist, the function returns an error.
	pub fn get_node(&self, inode: INode) -> EResult<&Box<dyn KernFSNode>> {
		if inode as usize >= self.nodes.len() {
			return Err(errno!(ENOENT));
		}

		self.nodes[inode as usize]
			.as_ref()
			.ok_or_else(|| errno!(ENOENT))
	}

	/// Returns a mutable reference to the node with inode `inode`.
	///
	/// If the node doesn't exist, the function returns an error.
	pub fn get_node_mut(&mut self, inode: INode) -> EResult<&mut Box<dyn KernFSNode>> {
		if inode as usize >= self.nodes.len() {
			return Err(errno!(ENOENT));
		}

		self.nodes[inode as usize]
			.as_mut()
			.ok_or_else(|| errno!(ENOENT))
	}

	/// Adds the given node `node` to the filesystem.
	///
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, node: Box<dyn KernFSNode>) -> EResult<INode> {
		if let Some(free_node) = self.free_nodes.pop() {
			// Using an existing slot
			self.nodes[free_node as usize] = Some(node);

			Ok(free_node)
		} else {
			// Allocating a new node slot
			let inode = self.nodes.len();
			self.nodes.push(Some(node))?;

			Ok(inode as _)
		}
	}

	/// Removes the node with inode `inode`.
	///
	/// If the node is a non-empty directory, its content is **NOT** removed.
	///
	/// If the node doesn't exist, the function does nothing.
	pub fn remove_node(&mut self, inode: INode) -> EResult<Option<Box<dyn KernFSNode>>> {
		if (inode as usize) < self.nodes.len() {
			let node = self.nodes.remove(inode as _);
			self.nodes.insert(inode as _, None)?;
			self.free_nodes.push(inode)?;

			return Ok(node);
		}

		Ok(None)
	}

	// TODO Clean
	/// Adds a file to the kernfs.
	///
	/// Arguments
	/// - `parent_inode` is the inode of the parent directory in which the file is inserted.
	/// - `node` is the node of the new file.
	/// - `name` is the name of the new file.
	pub fn add_file_inner<N: 'static + KernFSNode>(
		&mut self,
		parent_inode: INode,
		node: N,
		name: String,
	) -> EResult<File> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}

		let mode = node.get_mode();
		let uid = node.get_uid();
		let gid = node.get_gid();

		// Check parent exists
		self.get_node_mut(parent_inode)?;

		let inode = self.add_node(Box::new(node)?)?;
		let node = self.get_node_mut(inode)?;
		let mut content = node.get_content()?;
		let file_type = content.as_type();

		// Add `.` and `..` entries if the new file is a directory
		if let FileContent::Directory(ref mut entries) = &mut *content {
			let missing_cur = !entries.contains_key(b".".as_slice());
			let missing_parent = !entries.contains_key(b"..".as_slice());
			if missing_cur {
				entries.insert(
					b".".as_slice().try_into()?,
					DirEntry {
						inode,
						entry_type: FileType::Directory,
					},
				)?;
			}
			if missing_parent {
				entries.insert(
					b"..".as_slice().try_into()?,
					DirEntry {
						inode: parent_inode,
						entry_type: FileType::Directory,
					},
				)?;
			}

			// Increment after to prevent double borrow
			if missing_cur {
				let new_cnt = node.get_hard_links_count() + 1;
				node.set_hard_links_count(new_cnt);
			}
			if missing_parent {
				let parent = self.get_node_mut(parent_inode).unwrap();
				let new_cnt = parent.get_hard_links_count() + 1;
				parent.set_hard_links_count(new_cnt);
			}
		}

		let node = self.get_node_mut(inode)?;
		let content = oom::wrap(|| node.get_content().map_err(|_| AllocError)?.to_owned());
		let location = FileLocation::Filesystem {
			mountpoint_id: 0, // dummy value to be replaced
			inode,
		};
		let file = File::new(name.try_clone()?, uid, gid, mode, location, content)?;

		// Adding entry to parent
		let parent = self.get_node_mut(parent_inode).unwrap();
		let mut parent_content = parent.get_content()?;
		let parent_entries = match &mut *parent_content {
			FileContent::Directory(parent_entries) => parent_entries,
			_ => return Err(errno!(ENOENT)),
		};
		oom::wrap(|| {
			parent_entries.insert(
				name.try_clone()?,
				DirEntry {
					inode,
					entry_type: file_type,
				},
			)
		});

		Ok(file)
	}
}

impl Filesystem for KernFS {
	fn get_name(&self) -> &[u8] {
		self.name.as_bytes()
	}

	fn is_readonly(&self) -> bool {
		self.readonly
	}

	fn use_cache(&self) -> bool {
		false
	}

	fn get_root_inode(&self) -> INode {
		ROOT_INODE
	}

	fn get_stat(&self, _io: &mut dyn IO) -> EResult<Statfs> {
		Ok(Statfs {
			f_type: 0, // TODO
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

	fn get_inode(
		&mut self,
		_io: &mut dyn IO,
		parent: Option<INode>,
		name: &[u8],
	) -> EResult<INode> {
		// Getting the parent node
		let parent = parent.unwrap_or(ROOT_INODE);
		let parent = self.get_node_mut(parent)?;

		let FileContent::Directory(entries) = &mut *parent.get_content()? else {
			return Err(errno!(ENOENT));
		};
		entries
			.get(name)
			.map(|dirent| dirent.inode)
			.ok_or_else(|| errno!(ENOENT))
	}

	fn load_file(&mut self, _: &mut dyn IO, inode: INode, name: String) -> EResult<File> {
		let node = self.get_node_mut(inode)?;

		let file_location = FileLocation::Filesystem {
			mountpoint_id: 0, // dummy value to be replaced
			inode,
		};
		let content = node.get_content()?.to_owned()?;

		let mut file = File::new(
			name,
			node.get_uid(),
			node.get_gid(),
			node.get_mode(),
			file_location,
			content,
		)?;
		file.set_hard_links_count(node.get_hard_links_count());
		file.set_size(node.get_size());
		file.ctime = node.get_ctime();
		file.mtime = node.get_mtime();
		file.atime = node.get_atime();

		Ok(file)
	}

	fn add_file(
		&mut self,
		_: &mut dyn IO,
		parent_inode: INode,
		name: String,
		uid: Uid,
		gid: Gid,
		mode: Mode,
		content: FileContent,
	) -> EResult<File> {
		let node = DummyKernFSNode::new(mode, uid, gid, content);
		self.add_file_inner(parent_inode, node, name)
	}

	fn add_link(
		&mut self,
		_: &mut dyn IO,
		parent_inode: INode,
		name: &[u8],
		inode: INode,
	) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}

		// Checking the node exists
		self.get_node(inode)?;

		// Insert the new entry
		let parent = self.get_node_mut(parent_inode)?;
		let mut parent_content = parent.get_content()?;
		let entry_type = parent_content.as_type();
		let FileContent::Directory(entries) = &mut *parent_content else {
			return Err(errno!(ENOTDIR));
		};
		entries.insert(
			name.try_into()?,
			DirEntry {
				inode,
				entry_type,
			},
		)?;

		// Incrementing the number of links
		let node = self.get_node_mut(inode)?;
		let links = node.get_hard_links_count() + 1;
		node.set_hard_links_count(links);

		Ok(())
	}

	fn update_inode(&mut self, _: &mut dyn IO, file: &File) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}

		// Getting node
		let node = self.get_node_mut(file.get_location().get_inode())?;

		// Changing file size if it has been truncated
		// TODO node.truncate(file.get_size())?;

		// Updating file attributes
		node.set_uid(file.get_uid());
		node.set_gid(file.get_gid());
		node.set_mode(file.get_mode());
		node.set_ctime(file.ctime);
		node.set_mtime(file.mtime);
		node.set_atime(file.atime);

		Ok(())
	}

	fn remove_file(
		&mut self,
		_: &mut dyn IO,
		parent_inode: INode,
		name: &[u8],
	) -> EResult<(u16, INode)> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}

		// Getting directory entry
		let parent = self.get_node_mut(parent_inode)?;
		let FileContent::Directory(parent_entries) = &*parent.get_content()? else {
			return Err(errno!(ENOTDIR));
		};

		let inode = parent_entries
			.get(name)
			.ok_or_else(|| errno!(ENOENT))?
			.inode;
		let node = self.get_node_mut(inode)?;
		if let FileContent::Directory(entries) = &*node.get_content()? {
			if entries.len() > 2 {
				return Err(errno!(ENOTEMPTY));
			}
			if entries.iter().any(|(e, _)| e != "." && e != "..") {
				return Err(errno!(ENOTEMPTY));
			}
		}
		let is_dir = matches!(node.get_content()?.borrow(), FileContent::Directory(_));

		// Removing directory entry
		let parent = self.get_node_mut(parent_inode).unwrap();
		let mut content = parent.get_content()?;
		let FileContent::Directory(entries) = &mut *content else {
			unreachable!();
		};
		entries.remove(name);

		// If the node is a directory, decrement the number of hard links in the parent
		// (entry `..`)
		if is_dir {
			let parent = self.get_node_mut(parent_inode).unwrap();
			let links = parent.get_hard_links_count() - 1;
			parent.set_hard_links_count(links);
		}

		// If no link is left, remove the node
		let node = self.get_node_mut(inode)?;
		let links = node.get_hard_links_count() - 1;
		node.set_hard_links_count(links);
		if node.get_hard_links_count() == 0 {
			oom::wrap(|| self.remove_node(inode).map_err(|_| AllocError));
		}

		Ok((links, inode))
	}

	fn read_node(
		&mut self,
		_: &mut dyn IO,
		inode: INode,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		let node = self.get_node_mut(inode)?;
		Ok(node.read(off, buf)?.0)
	}

	fn write_node(&mut self, _: &mut dyn IO, inode: INode, off: u64, buf: &[u8]) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}

		let node = self.get_node_mut(inode)?;
		node.write(off, buf)?;
		Ok(())
	}
}
