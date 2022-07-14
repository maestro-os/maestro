//! The procfs is a virtual filesystem which provides informations about processes.

pub mod mount;
pub mod proc_dir;
pub mod self_link;

use crate::errno::Errno;
use crate::file::DirEntry;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::Statfs;
use crate::file::path::Path;
use crate::process::pid::Pid;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::ptr::SharedPtr;
use proc_dir::ProcDir;
use self_link::SelfNode;
use super::Filesystem;
use super::FilesystemType;
use super::kernfs::KernFS;
use super::kernfs::node::DummyKernFSNode;

/// Structure representing the procfs.
/// On the inside, the procfs works using a kernfs.
pub struct ProcFS {
	/// The kernfs.
	fs: KernFS,

	/// The list of registered processes with their directory's inode.
	procs: HashMap<Pid, INode>,
}

impl ProcFS {
	/// Creates a new instance.
	/// `readonly` tells whether the filesystem is readonly.
	/// `mountpath` is the path at which the filesystem is mounted.
	pub fn new(readonly: bool, mountpath: Path) -> Result<Self, Errno> {
		let mut fs = Self {
			fs: KernFS::new(String::from(b"procfs")?, readonly, mountpath)?,

			procs: HashMap::new(),
		};

		let mut root_entries = HashMap::new();

		// Creating /proc/self
		let self_node = SelfNode {};
		let self_inode = fs.fs.add_node(Box::new(self_node)?)?;
		root_entries.insert(String::from(b"self")?, DirEntry {
			inode: self_inode,
			entry_type: FileType::Link,
		})?;

		// Creating /proc/mounts
		let mount_node = DummyKernFSNode::new(0o444, 0, 0,
			FileContent::Link(String::from(b"self/mounts")?), None);
		let mount_inode = fs.fs.add_node(Box::new(mount_node)?)?;
		root_entries.insert(String::from(b"mounts")?, DirEntry {
			inode: mount_inode,
			entry_type: FileType::Link,
		})?;

		// TODO Iterate on processes to register them

		// Adding the root node
		let root_node = DummyKernFSNode::new(0o555, 0, 0, FileContent::Directory(root_entries),
			None);
		fs.fs.set_root(Box::new(root_node)?)?;

		Ok(fs)
	}

	/// Adds a process with the given PID `pid` to the filesystem.
	pub fn add_process(&mut self, pid: Pid) -> Result<(), Errno> {
		// Creating the process's node
		let proc_node = ProcDir::new(pid, &mut self.fs)?;
		self.fs.add_node(Box::new(proc_node)?)?;

		// Inserting the process's node at the root of the filesystem
		// TODO

		Ok(())
	}

	/// Removes the process with pid `pid` from the filesystem.
	/// If the process doesn't exist, the function does nothing.
	pub fn remove_process(&mut self, _pid: Pid) -> Result<(), Errno> {
		// TODO
		todo!();
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

	fn get_stat(&self, io: &mut dyn IO) -> Result<Statfs, Errno> {
		self.fs.get_stat(io)
	}

	fn get_root_inode(&self, io: &mut dyn IO) -> Result<INode, Errno> {
		self.fs.get_root_inode(io)
	}

	fn get_inode(&mut self, io: &mut dyn IO, parent: Option<INode>, name: &String)
		-> Result<INode, Errno> {
		self.fs.get_inode(io, parent, name)
	}

	fn load_file(&mut self, io: &mut dyn IO, inode: INode, name: String) -> Result<File, Errno> {
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

	fn create_filesystem(&self, _io: &mut dyn IO) -> Result<SharedPtr<dyn Filesystem>, Errno> {
		Ok(SharedPtr::new(ProcFS::new(false, Path::root())?)?)
	}

	fn load_filesystem(&self, _io: &mut dyn IO, mountpath: Path, readonly: bool)
		-> Result<SharedPtr<dyn Filesystem>, Errno> {
		Ok(SharedPtr::new(ProcFS::new(readonly, mountpath)?)?)
	}
}
