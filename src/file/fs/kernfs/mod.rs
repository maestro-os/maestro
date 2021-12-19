//! Kernfs implements utilities allowing to create a virtual filesystem.

use crate::device::Device;
use crate::errno::Errno;
use crate::file::File;
use crate::file::FileType;
use crate::file::INode;
use crate::file::fs::Filesystem;
use crate::file::path::Path;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;

/// Trait representing a node in a kernfs.
pub trait KernFSNode: IO {
	/// Returns the type of the node.
	fn get_type(&self) -> FileType;

	/// Returns the list of entries in the node.
	fn get_entries(&self) -> &HashMap<String, Self> where Self: Sized;

	/// Returns the entry with name `name`.
	fn get_entry(&self, name: &String) -> Option<&Self> where Self: Sized {
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
	root: Box<dyn KernFSNode>,
}

impl KernFS {
	/// Creates a new instance.
	/// `name` is the name of the filesystem.
	/// `readonly` tells whether the filesystem is readonly.
	/// `root` is the root of the filesystem.
	pub fn new(name: String, readonly: bool, root: Box<dyn KernFSNode>) -> Self {
		Self {
			name,
			readonly,

			root,
		}
	}

	/// Returns the root node of the filesystem.
	pub fn root(&self) -> &dyn KernFSNode {
		&*self.root
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

	fn get_inode(&mut self, dev: &mut Device, path: Path) -> Result<INode, Errno> {
		// TODO
		todo!();
	}

	fn load_file(&mut self, dev: &mut Device, inode: INode, name: String) -> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn add_file(&mut self, dev: &mut Device, parent_inode: INode, file: File)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn remove_file(&mut self, dev: &mut Device, parent_inode: INode, name: &String)
		-> Result<(), Errno> {
		// TODO
		todo!();
	}

	fn read_node(&mut self, dev: &mut Device, inode: INode, off: u64, buf: &mut [u8])
		-> Result<usize, Errno> {
		// TODO
		todo!();
	}

	fn write_node(&mut self, dev: &mut Device, inode: INode, off: u64, buf: &[u8])
		-> Result<(), Errno> {
		// TODO
		todo!();
	}
}
