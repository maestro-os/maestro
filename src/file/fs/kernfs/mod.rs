//! Kernfs implements utilities allowing to create a virtual filesystem.

pub mod directory;
pub mod node;

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::Filesystem;
use crate::file::path::Path;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::ptr::SharedPtr;
use node::KernFSNode;

/// The index of the root inode.
const ROOT_INODE: INode = 0;

/// Structure representing a kernel file system.
pub struct KernFS {
	/// The name of the filesystem.
	name: String,
	/// Tells whether the filesystem is readonly.
	readonly: bool,

	/// The list of nodes of the filesystem. The index in this vector is the inode.
	nodes: Vec<Option<SharedPtr<dyn KernFSNode>>>,
	/// A list of free inodes.
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

	/// Sets the root node of the filesystem.
	pub fn set_root(&mut self, root: Option<SharedPtr<dyn KernFSNode>>) -> Result<(), Errno> {
		if self.nodes.is_empty() {
			self.nodes.push(root)?;
		} else {
			self.nodes[ROOT_INODE as _] = root;
		}

		Ok(())
	}

	/// Adds the given node `node` at the given path `path`.
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, path: &Path, _node: SharedPtr<dyn KernFSNode>)
		-> Result<INode, Errno> {
		let mut parent_path = path.failable_clone()?;
		parent_path.pop();

		// TODO Get parent inode
		// TODO Add file
		todo!();
	}

	/// Removes the node with inode `inode`.
	pub fn remove_node(&mut self, _inode: INode) -> Result<(), Errno> {
		// If the previous node has entries, free everything recursively
		// TODO (Handles cases where multiple links are present)

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

	fn get_root_inode(&self, _io: &mut dyn IO) -> Result<INode, Errno> {
		Ok(ROOT_INODE)
	}

	fn get_inode(&mut self, _io: &mut dyn IO, parent: Option<INode>, name: &String)
		-> Result<INode, Errno> {
		let parent_inode = parent.unwrap_or(ROOT_INODE);

		// Getting the parent node
		if parent_inode as usize >= self.nodes.len() {
			return Err(errno!(ENOENT));
		}
		let parent_mutex = self.nodes[parent_inode as _].as_ref().ok_or(errno!(ENOENT))?;
		let parent_guard = parent_mutex.lock();
		let parent_node = parent_guard.get();

		parent_node.get_entry(name).map(| (inode, _) | inode)
	}

	fn load_file(&mut self, _: &mut dyn IO, _inode: INode, _name: String)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn add_file(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: String, _uid: Uid,
		_gid: Gid, _mode: Mode, _content: FileContent) -> Result<File, Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn add_link(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: &String, _inode: INode)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn update_inode(&mut self, _io: &mut dyn IO, _file: &File) -> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn remove_file(&mut self, _: &mut dyn IO, _parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn read_node(&mut self, _: &mut dyn IO, _inode: INode, _off: u64, _buf: &mut [u8])
		-> Result<u64, Errno> {
		// TODO
		todo!();
	}

	fn write_node(&mut self, _: &mut dyn IO, _inode: INode, _off: u64, _buf: &[u8])
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}
}
