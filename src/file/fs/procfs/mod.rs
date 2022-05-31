//! The procfs is a virtual filesystem which provides informations about processes.

pub mod mount;

use crate::errno::Errno;
use crate::file::DirEntry;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::path::Path;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use mount::ProcFSMountIO;
use super::Filesystem;
use super::FilesystemType;
use super::kernfs::KernFS;
use super::kernfs::node::KernFSNode;

/// Structure representing the procfs.
/// On the inside, the procfs works using a kernfs.
pub struct ProcFS {
	/// The kernfs.
	fs: KernFS,
}

impl ProcFS {
	/// Creates a new instance.
	/// `readonly` tells whether the filesystem is readonly.
	/// `mountpath` is the path at which the filesystem is mounted.
	pub fn new(readonly: bool, mountpath: Path) -> Result<Self, Errno> {
		let mut fs = Self {
			fs: KernFS::new(String::from(b"procfs")?, readonly, mountpath),
		};

		let mut root_entries = HashMap::new();

		// Creating /proc/mounts
		let mount_inode = fs.fs.add_node(KernFSNode::new(0o666, 0, 0,
			FileContent::Regular, Some(Box::new(ProcFSMountIO {})?)))?;
		root_entries.insert(String::from(b"mounts")?, DirEntry {
			inode: mount_inode,
			entry_type: FileType::Regular,
		})?;

		// Adding the root node
		let root_node = KernFSNode::new(0o666, 0, 0, FileContent::Directory(root_entries), None);
		fs.fs.set_root(Some(root_node))?;

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

	fn get_root_inode(&self, io: &mut dyn IO) -> Result<INode, Errno> {
		self.fs.get_root_inode(io)
	}

	fn get_inode(&mut self, io: &mut dyn IO, parent: Option<INode>, name: &String)
		-> Result<INode, Errno> {
		self.fs.get_inode(io, parent, name)
	}

	fn load_file(&mut self, io: &mut dyn IO, inode: INode, name: String)
		-> Result<File, Errno> {
		self.fs.load_file(io, inode, name)
	}

	fn add_file(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: String, _uid: Uid,
		_gid: Gid, _mode: Mode, _content: FileContent) -> Result<File, Errno> {
		Err(errno!(EPERM))
	}

	fn add_link(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: &String,
		_inode: INode) -> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn update_inode(&mut self, _io: &mut dyn IO, _file: &File) -> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn remove_file(&mut self, _io: &mut dyn IO, _parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		Err(errno!(EPERM))
	}

	fn read_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &mut [u8])
		-> Result<u64, Errno> {
		self.fs.read_node(io, inode, off, buf)
	}

	fn write_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &[u8])
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
		Ok(Box::new(ProcFS::new(false, Path::root())?)?)
	}

	fn load_filesystem(&self, _io: &mut dyn IO, mountpath: Path, readonly: bool)
		-> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(ProcFS::new(readonly, mountpath)?)?)
	}
}
