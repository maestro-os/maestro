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
use crate::file::inode::INode;
use crate::file::path::Path;
use crate::time::Timestamp;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::ptr::SharedPtr;

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
	fn get_entries(&self) -> &HashMap<String, SharedPtr<dyn KernFSNode>>;

	/// Returns the inode of the entry with name `name`.
	/// If the entry doesn't exist, the function returns an error.
	/// Reimplementing this function is highly recommended since the default implementation is
	/// using get_entries, which can be fairly slow depending on the amount of elements in the
	/// node.
	fn get_entry(&self, name: &String) -> Result<SharedPtr<dyn KernFSNode>, Errno> {
		Ok(self.get_entries().get(name).ok_or(errno!(ENOENT))?.clone())
	}
}

/// Structure wrapping a reference to a kernfs node into an inode.
pub struct KernFSINode {
	/// The node.
	pub node: SharedPtr<dyn KernFSNode>,
}

impl INode for KernFSINode {}

/// Structure representing a kernel file system.
pub struct KernFS {
	/// The name of the filesystem.
	name: String,
	/// Tells whether the filesystem is readonly.
	readonly: bool,

	/// The root node of the filesystem.
	root_node: Option<SharedPtr<dyn KernFSNode>>,
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
	pub fn set_root(&mut self, root: Option<SharedPtr<dyn KernFSNode>>) {
		self.root_node = root;
	}

	/// Adds the given node `node` at the given path `path`.
	/// The function returns the allocated inode.
	pub fn add_node(&mut self, path: &Path, _node: SharedPtr<dyn KernFSNode>)
		-> Result<Box<dyn INode>, Errno> {
		let mut parent_path = path.failable_clone()?;
		parent_path.pop();

		// TODO Get parent inode
		// TODO Add file
		todo!();
	}

	/// Removes the node with inode `inode`.
	pub fn remove_node(&mut self, _inode: &Box<dyn INode>) -> Result<(), Errno> {
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

	fn get_root_inode(&self, _io: &mut dyn IO) -> Result<Box<dyn INode>, Errno> {
		// TODO
		todo!();
	}

	fn get_inode(&mut self, _io: &mut dyn IO, parent: Option<&Box<dyn INode>>, name: &String)
		-> Result<Box<dyn INode>, Errno> {
		let parent_mutex = parent.map(| p | {
				&<dyn Any>::downcast_ref::<KernFSINode>(p.as_ref()).unwrap().node
			})
			.or_else(|| self.root_node.as_ref())
			.ok_or(errno!(ENOENT))?;
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get();

		Ok(Box::new(KernFSINode {
			node: parent.get_entry(name)?
		})?)
	}

	fn load_file(&mut self, _: &mut dyn IO, _inode: &Box<dyn INode>, _name: String)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn add_file(&mut self, _io: &mut dyn IO, _parent_inode: &Box<dyn INode>, _name: String,
		_uid: Uid, _gid: Gid, _mode: Mode, _content: FileContent) -> Result<File, Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn add_link(&mut self, _io: &mut dyn IO, _parent_inode: &Box<dyn INode>, _name: &String,
		_inode: &Box<dyn INode>) -> Result<(), Errno> {
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

	fn remove_file(&mut self, _: &mut dyn IO, _parent_inode: &Box<dyn INode>, _name: &String)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}

	fn read_node(&mut self, _: &mut dyn IO, _inode: &Box<dyn INode>, _off: u64, _buf: &mut [u8])
		-> Result<u64, Errno> {
		// TODO
		todo!();
	}

	fn write_node(&mut self, _: &mut dyn IO, _inode: &Box<dyn INode>, _off: u64, _buf: &[u8])
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno!(EROFS));
		}

		// TODO
		todo!();
	}
}
