//! Kernfs implements utilities allowing to create a virtual filesystem.

pub mod node;

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
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
use node::KernFSNode;

/// The index of the root inode.
const ROOT_INODE: INode = 0;

/// Structure representing a kernel file system.
pub struct KernFS {
	/// The name of the filesystem.
	name: String,
	/// Tells whether the filesystem is readonly.
	readonly: bool,

	/// The path at which the filesystem is mounted.
	mountpath: Path,

	/// The list of nodes of the filesystem. The index in this vector is the inode.
	nodes: Vec<Option<KernFSNode>>,
	/// A list of free inodes.
	free_nodes: Vec<INode>,
}

impl KernFS {
	/// Creates a new instance.
	/// `name` is the name of the filesystem.
	/// `readonly` tells whether the filesystem is readonly.
	/// `mountpath` is the path at which the filesystem is mounted.
	pub fn new(name: String, readonly: bool, mountpath: Path) -> Self {
		Self {
			name,
			readonly,

			mountpath,

			nodes: Vec::new(),
			free_nodes: Vec::new(),
		}
	}

	/// Sets the root node of the filesystem.
	pub fn set_root(&mut self, root: Option<KernFSNode>) -> Result<(), Errno> {
		if self.nodes.is_empty() {
			self.nodes.push(root)?;
		} else {
			self.nodes[ROOT_INODE as _] = root;
		}

		Ok(())
	}

	/// Adds the given node `node` to the filesystem.
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, node: KernFSNode) -> Result<INode, Errno> {
		// TODO Use the free nodes list
		let inode = self.nodes.len();
		self.nodes.push(Some(node))?;

		Ok(inode as _)
	}

	/// Removes the node with inode `inode`.
	pub fn remove_node(&mut self, inode: INode) -> Result<(), Errno> {
		if let Some(node) = &self.nodes[inode as _] {
			// If the node is a non-empty directory, return an error
			match node.get_content() {
				FileContent::Directory(entries) if !entries.is_empty() => {
					return Err(errno!(ENOTEMPTY));
				},
				_ => {},
			}

			self.nodes[inode as _] = None;
			// TODO Add to free list
		}

		Ok(())
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
		let parent_node = self.nodes[parent_inode as _].as_ref().ok_or_else(|| errno!(ENOENT))?;

		match parent_node.get_content() {
			FileContent::Directory(entries) => {
				entries.get(name)
					.map(| dirent | dirent.inode)
					.ok_or_else(|| errno!(ENOENT))
			},
			_ => Err(errno!(ENOENT)),
		}
	}

	fn load_file(&mut self, _: &mut dyn IO, inode: INode, name: String)
		-> Result<File, Errno> {
		if inode as usize >= self.nodes.len() {
			return Err(errno!(ENOENT));
		}
		let node = self.nodes[inode as _].as_ref().ok_or_else(|| errno!(ENOENT))?;

		let file_location = FileLocation::new(self.mountpath.failable_clone()?, inode);
		let file_content = node.get_content().failable_clone()?;

		let mut file = File::new(name, node.get_uid(), node.get_gid(), node.get_mode(),
			file_location, file_content)?;
		file.set_hard_links_count(node.get_hard_links_count());
		file.set_size(node.get_size());
		file.set_ctime(node.get_ctime());
		file.set_mtime(node.get_mtime());
		file.set_atime(node.get_atime());

		Ok(file)
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
