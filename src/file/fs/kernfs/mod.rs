//! Kernfs implements utilities allowing to create a virtual filesystem.

use core::any::Any;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
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
	fn get_entries(&self) -> Result<HashMap<String, Box<dyn KernFSNode>>, Errno>;

	/// Returns the inode of the entry with name `name`.
	/// If the entry doesn't exist, the function returns an error.
	fn get_entry(&self, name: &String) -> Result<&Box<dyn KernFSNode>, Errno> {
		self.get_entries()?.get(name).ok_or(errno!(ENOENT))
	}
}

/// Structure representing a kernel file system.
pub struct KernFS {
	/// The name of the filesystem.
	name: String,
	/// Tells whether the filesystem is readonly.
	readonly: bool,

	/// The root node of the filesystem.
	root_node: Option<Box<dyn KernFSNode>>,
}

impl KernFS {
	/// Creates a new instance.
	/// `name` is the name of the filesystem.
	/// `readonly` tells whether the filesystem is readonly.
	pub fn new(name: String, readonly: bool) -> Self {
		Self {
			name,
			readonly,

			root_node: None,
		}
	}

	/// Sets the root node of the filesystem.
	pub fn set_root(&mut self, root: Option<Box<dyn KernFSNode>>) {
		self.root_node = root;
	}

	/// Adds the given node `node` at the given path `path`.
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, path: &Path, _node: Box<dyn KernFSNode>)
		-> Result<Box<dyn Any>, Errno> {
		let mut parent_path = path.failable_clone()?;
		parent_path.pop();

		// TODO Get parent inode
		// TODO Add file
		todo!();
	}

	/// Removes the node with inode `inode`.
	pub fn remove_node(&mut self, _inode: Box<dyn Any>) -> Result<(), Errno> {
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

	fn get_inode(&mut self, _io: &mut dyn IO, parent: Option<Box<dyn Any>>, name: Option<&String>)
		-> Result<Box<dyn Any>, Errno> {
		let parent = parent.map(| p | p.downcast_ref::<Box<dyn KernFSNode>>())
			.unwrap_or_else(|| self.root_node.map(| r | &r)).ok_or(errno!(ENOENT))?;

		if let Some(name) = name {
			Ok(*parent.get_entry(name)?)
		} else {
			Ok(*parent as _)
		}
	}

	fn load_file(&mut self, _: &mut dyn IO, inode: Box<dyn Any>, _name: String)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn add_file(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn Any>, _name: String,
		_uid: Uid, _gid: Gid, _mode: Mode, _content: FileContent) -> Result<File, Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn add_link(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn Any>, _name: &String,
		_inode: Box<dyn Any>) -> Result<(), Errno> {
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

	fn remove_file(&mut self, _: &mut dyn IO, _parent_inode: Box<dyn Any>, _name: &String)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn read_node(&mut self, _: &mut dyn IO, inode: Box<dyn Any>, off: u64, buf: &mut [u8])
		-> Result<u64, Errno> {
		// TODO
		todo!();
	}

	fn write_node(&mut self, _: &mut dyn IO, inode: Box<dyn Any>, off: u64, buf: &[u8])
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}
}
