//! Kernfs implements utilities allowing to create a virtual filesystem.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileType;
use crate::file::INode;
use crate::file::fs::Filesystem;
use crate::file::path::Path;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// Trait representing a node in a kernfs.
pub trait KernFSNode: IO {
	/// Returns the type of the node.
	fn get_type(&self) -> FileType;

	/// Returns the list of entries in the node.
	fn get_entries(&self) -> &HashMap<String, INode>;

	/// Returns the inode of the entry with name `name`.
	fn get_entry(&self, name: &String) -> Option<&INode> {
		self.get_entries().get(name)
	}
}

/// Structure representing a kernel file system.
pub struct KernFS {
	/// The name of the filesystem.
	name: String,
	/// Tells whether the filesystem is readonly.
	readonly: bool,

	/// The root node of the filesystem.
	nodes: Vec<Option<Box<dyn KernFSNode>>>,
	/// The list of free inodes in the nodes list.
	free_nodes: Vec<INode>,
}

impl KernFS {
	/// Creates a new instance.
	/// `name` is the name of the filesystem.
	/// `readonly` tells whether the filesystem is readonly.
	pub fn new(name: String, readonly: bool) -> Self {
		Self {
			name,
			readonly,

			nodes: Vec::new(),
			free_nodes: Vec::new(),
		}
	}

	/// Allocates an inode.
	fn alloc_inode(&mut self) -> Result<INode, Errno> {
		if let Some(inode) = self.free_nodes.pop() {
			Ok(inode)
		} else {
			self.nodes.push(None)?;
			Ok((self.nodes.len() - 1) as _)
		}
	}

	/// Frees the inode `inode`.
	fn free_inode(&mut self, inode: INode) -> Result<(), Errno> {
		if inode as usize == self.nodes.len() - 1 {
			self.nodes.pop();
		} else {
			self.nodes[inode as _] = None;
			self.free_nodes.push(inode)?;
		}

		Ok(())
	}

	/// Adds the given node `node` at the given path `path`.
	pub fn add_node(&mut self, _node: Box<dyn KernFSNode>, _path: &Path) -> Result<(), Errno> {
		// TODO
		todo!();
	}

	/// Removes the node at the given path `path`.
	pub fn remove_node(&mut self, _path: &Path) -> Result<(), Errno> {
		// TODO
		todo!();
	}
}

impl Filesystem for KernFS {
	fn get_name(&self) -> &[u8] {
		self.name.as_bytes()
	}

	fn is_readonly(&self) -> bool {
		self.readonly
	}

	fn must_cache(&self) -> bool {
		false
	}

	fn get_inode(&mut self, _: &mut dyn IO, path: Path) -> Result<INode, Errno> {
		// The current inode, initialized with the root node
		let mut inode: INode = 0;

		for i in 0..path.get_elements_count() {
			// Checking the node exists
			if self.nodes.is_empty() {
				return Err(errno::ENOENT);
			}

			// The current node
			let node = self.nodes[inode as _].as_ref().ok_or(errno::ENOENT)?;
			inode = *node.get_entry(&path[i]).ok_or(errno::ENOENT)?;
		}

		Ok(inode as _)
	}

	fn load_file(&mut self, _: &mut dyn IO, inode: INode, _name: String) -> Result<File, Errno> {
		let _node = self.nodes[inode as _].as_ref().ok_or(errno::ENOENT)?;

		// TODO
		todo!();
	}

	fn add_file(&mut self, _: &mut dyn IO, _parent_inode: INode, _file: File)
		-> Result<File, Errno> {
		if self.readonly {
			return Err(errno::EROFS);
		}

		// TODO
		todo!();
	}

	fn remove_file(&mut self, _: &mut dyn IO, _parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno::EROFS);
		}

		// TODO
		todo!();
	}

	fn read_node(&mut self, _: &mut dyn IO, _inode: INode, _off: u64, _buf: &mut [u8])
		-> Result<usize, Errno> {
		// TODO
		todo!();
	}

	fn write_node(&mut self, _: &mut dyn IO, _inode: INode, _off: u64, _buf: &[u8])
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno::EROFS);
		}

		// TODO
		todo!();
	}
}
