//! The procfs is a virtual filesystem which provides informations about processes.

pub mod mount;
pub mod root;

use core::any::Any;
use crate::errno::Errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::Filesystem;
use crate::file::fs::FilesystemType;
use crate::file::fs::kernfs::KernFS;
use crate::file::path::Path;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use root::ProcFSRoot;

/// Structure representing the procfs.
/// On the inside, the procfs works using a kernfs.
pub struct ProcFS {
	/// The kernfs.
	fs: KernFS,
}

impl ProcFS {
	/// Creates a new instance.
	/// `readonly` tells whether the filesystem is readonly.
	pub fn new(readonly: bool) -> Result<Self, Errno> {
		let mut fs = Self {
			fs: KernFS::new(String::from(b"procfs")?, readonly),
		};

		// Adding the root node
		let root_node = ProcFSRoot::new();
		fs.fs.set_root(Some(Box::new(root_node)?));

		Ok(fs)
	}
}

impl Filesystem for ProcFS {
	fn get_name(&self) -> &[u8] {
		self.fs.get_name()
	}

	fn is_readonly(&self) -> bool {
		self.fs.is_readonly()
	}

	fn must_cache(&self) -> bool {
		self.fs.must_cache()
	}

	fn get_inode(&mut self, io: &mut dyn IO, parent: Option<Box<dyn Any>>, name: Option<&String>)
		-> Result<Box<dyn Any>, Errno> {
		self.fs.get_inode(io, parent, name)
	}

	fn load_file(&mut self, io: &mut dyn IO, inode: Box<dyn Any>, name: String)
		-> Result<File, Errno> {
		self.fs.load_file(io, inode, name)
	}

	fn add_file(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn Any>, _name: String, _uid: Uid,
		_gid: Gid, _mode: Mode, _content: FileContent) -> Result<File, Errno> {
		Err(errno!(EPERM))
	}

	fn add_link(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn Any>, _name: &String,
		_inode: Box<dyn Any>) -> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn update_inode(&mut self, _io: &mut dyn IO, _file: &File) -> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn remove_file(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn Any>, _name: &String)
		-> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn read_node(&mut self, io: &mut dyn IO, inode: Box<dyn Any>, off: u64, buf: &mut [u8])
		-> Result<u64, Errno> {
		self.fs.read_node(io, inode, off, buf)
	}

	fn write_node(&mut self, io: &mut dyn IO, inode: Box<dyn Any>, off: u64, buf: &[u8])
		-> Result<(), Errno> {
		self.fs.write_node(io, inode, off, buf)
	}
}

/// Structure representing the procfs file system type.
pub struct ProcFsType {}

impl FilesystemType for ProcFsType {
	fn get_name(&self) -> &[u8] {
		b"procfs"
	}

	fn detect(&self, _io: &mut dyn IO) -> Result<bool, Errno> {
		Ok(false)
	}

	fn create_filesystem(&self, _io: &mut dyn IO) -> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(ProcFS::new(false)?)?)
	}

	fn load_filesystem(&self, _io: &mut dyn IO, _mountpath: Path, readonly: bool)
		-> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(ProcFS::new(readonly)?)?)
	}
}
