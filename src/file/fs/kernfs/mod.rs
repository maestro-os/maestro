//! Kernfs implements utilities allowing to create a virtual filesystem.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::Filesystem;
use crate::file::path::Path;
use crate::time::Timestamp;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// The inode index for the root directory.
pub const ROOT_INODE: INode = 0;

/// Trait representing a node in a kernfs.
pub trait KernFSNode: IO {
	/// Returns the type of the node.
	fn get_type(&self) -> FileType;

	/// Returns the permissions of the file.
	fn get_mode(&self) -> Mode;
	/// Sets the permissions of the file.
	fn set_mode(&mut self, mode: Mode);

	/// Returns the UID of the file's owner.
	fn get_uid(&self) -> Uid;
	/// Sets the UID of the file's owner.
	fn set_uid(&mut self, uid: Uid);
	/// Returns the GID of the file's owner.
	fn get_gid(&self) -> Gid;
	/// Sets the GID of the file's owner.
	fn set_gid(&mut self, gid: Gid);

	/// Returns the timestamp of the last access to the file.
	fn get_atime(&self) -> Timestamp;
	/// Sets the timestamp of the last access to the file.
	fn set_atime(&mut self, ts: Timestamp);

	/// Returns the timestamp of the last modification of the file's metadata.
	fn get_ctime(&self) -> Timestamp;
	/// Sets the timestamp of the last modification of the file's metadata.
	fn set_ctime(&mut self, ts: Timestamp);

	/// Returns the timestamp of the last modification of the file's content.
	fn get_mtime(&self) -> Timestamp;
	/// Sets the timestamp of the last modification of the file's content.
	fn set_mtime(&mut self, ts: Timestamp);

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

	/// The list of nodes in the filesystem. The index is the inode.
	nodes: Vec<Option<Box<dyn KernFSNode>>>,
	/// The sorted list of free inodes in the nodes list.
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

			// Inserting the node in the free nodes list, sorted
			let r = self.free_nodes.binary_search(&inode);
			let i = match r {
				Ok(i) => i,
				Err(i) => i,
			};
			self.free_nodes.insert(i, inode)?;
		}

		Ok(())
	}

	/// Sets the node `node` at the given inode `inode`.
	/// If a node already exist at the given inode, the function shall replace it.
	/// If the previous node has entries, each of them shall be removed recursively to
	/// avoid leaks.
	/// If `inode` is greater than the number of nodes in the nodes list, the function fails.
	pub fn set_node(&mut self, inode: INode, node: Box<dyn KernFSNode>) -> Result<(), Errno> {
		// Removing the inode from the free list if present
		let r = self.free_nodes.binary_search(&inode);
		match r {
			Ok(i) => {
				self.free_nodes.remove(i);
			},
			Err(_) => {},
		}

		// Setting the node
		if (inode as usize) < self.nodes.len() {
			// If the previous node has entries, free everything recursively
			if self.nodes[inode as _].is_some() {
				self.remove_node(inode)?;
			}

			self.nodes[inode as _] = Some(node);
			Ok(())
		} else if (inode as usize) == self.nodes.len() {
			self.nodes.push(Some(node))?;
			Ok(())
		} else {
			Err(errno!(EINVAL))
		}
	}

	/// Adds the given node `node` at the given path `path`.
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, path: &Path, _node: Box<dyn KernFSNode>) -> Result<INode, Errno> {
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
		/*for (_, inode) in old.get_entries().iter() {
			self.remove_node(*inode);
		}*/

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

	fn get_inode(&mut self, _io: &mut dyn IO, parent: Option<INode>, name: Option<&String>)
		-> Result<INode, Errno> {
		if self.nodes.is_empty() {
			return Err(errno!(ENOENT));
		}

		let parent_inode = parent.unwrap_or(ROOT_INODE);

		if let Some(name) = name {
			let parent = self.nodes[parent_inode as _].as_ref().ok_or(errno!(ENOENT))?;
			let inode = *parent.get_entry(name).ok_or(errno!(ENOENT))?;
			Ok(inode)
		} else {
			Ok(parent_inode)
		}
	}

	fn load_file(&mut self, _: &mut dyn IO, inode: INode, _name: String) -> Result<File, Errno> {
		let _node = self.nodes[inode as _].as_ref().ok_or(errno!(ENOENT))?;

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

	fn read_node(&mut self, _: &mut dyn IO, inode: INode, off: u64, buf: &mut [u8])
		-> Result<u64, Errno> {
		if (inode as usize) >= self.nodes.len() || self.nodes[inode as _].is_none() {
			return Err(errno!(ENOENT));
		}

		self.nodes[inode as _].as_ref().unwrap().read(off, buf)
	}

	fn write_node(&mut self, _: &mut dyn IO, inode: INode, off: u64, buf: &[u8])
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		if (inode as usize) >= self.nodes.len() || self.nodes[inode as _].is_none() {
			return Err(errno!(ENOENT));
		}

		self.nodes[inode as _].as_mut().unwrap().write(off, buf)?;
		Ok(())
	}
}
