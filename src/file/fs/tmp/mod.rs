//! Tmpfs (Temporary file system) is, as its name states a temporary filesystem. The files are
//! stored on the kernel's memory and thus are removed when the filesystem is unmounted.

use core::mem::size_of;
use crate::errno;
use crate::file::Errno;
use crate::file::File;
use crate::file::FileType;
use crate::file::INode;
use crate::file::fs::Filesystem;
use crate::file::fs::kernfs::KernFSNode;
use crate::file::path::Path;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// The default maximum amount of memory the filesystem can use in bytes.
const DEFAULT_MAX_SIZE: usize = 512 * 1024 * 1024;
/// The inode index for the root directory.
const ROOT_INODE: usize = 0;

/// Structure representing a file in a tmpfs.
pub struct TmpFSFile {
	/// The type of the file.
	type_: FileType,

	// TODO
}

impl TmpFSFile {
	/// Creates a new instance.
	/// `type_` is the file type.
	pub fn new(type_: FileType) -> Self {
		Self {
			type_,
		}
	}

	/// Returns the size used by the file in bytes.
	pub fn get_used_size(&self) -> usize {
		// TODO Add size of content
		size_of::<Self>()
	}
}

impl KernFSNode for TmpFSFile {
	fn get_type(&self) -> FileType {
		self.type_
	}

	fn get_entries(&self) -> &HashMap<String, INode> {
		// TODO
		todo!();
	}
}

impl IO for TmpFSFile {
	fn get_size(&self) -> u64 {
		// TODO
		todo!();
	}

	fn read(&self, _offset: u64, _buff: &mut [u8]) -> Result<usize, Errno> {
		// TODO
		todo!();
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<usize, Errno> {
		// TODO
		todo!();
	}
}

/// Structure representing the temporary file system.
pub struct TmpFS {
	/// The maximum amount of memory in bytes the filesystem can use.
	max_size: usize,
	/// The currently used amount of memory in bytes.
	size: usize,

	/// The files, ordered by inode number.
	files: Vec<TmpFSFile>,
}

impl TmpFS {
	/// Creates a new instance.
	/// `max_size` is the maximum amount of memory the filesystem can use in bytes.
	pub fn new(max_size: usize) -> Result<Self, Errno> {
		let mut fs = Self {
			max_size,
			size: 0,

			files: Vec::new(),
		};
		fs.files.insert(ROOT_INODE, TmpFSFile::new(FileType::Directory))?;
		fs.increase_size(fs.files[0].get_used_size())?;

		Ok(fs)
	}

	/// Increases the total size of the fs by `s`. If the size is too large, the function returns
	/// an error.
	fn increase_size(&mut self, s: usize) -> Result<(), Errno> {
		if self.size + s < self.max_size {
			self.size += s;
			Ok(())
		} else {
			Err(errno::ENOSPC)
		}
	}
}

impl Filesystem for TmpFS {
	fn get_name(&self) -> &[u8] {
		b"tmpfs"
	}

	fn is_readonly(&self) -> bool {
		false
	}
	fn must_cache(&self) -> bool {
		false
	}

	fn get_inode(&mut self, _dev: &mut dyn IO, _path: Path) -> Result<INode, Errno> {
		let _root = &self.files[ROOT_INODE];

		// TODO
		todo!();
	}

	fn load_file(&mut self, _dev: &mut dyn IO, _inode: INode, _name: String)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn add_file(&mut self, _dev: &mut dyn IO, _parent_inode: INode, _file: File)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn remove_file(&mut self, _dev: &mut dyn IO, _parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		// TODO
		todo!();
	}

	fn read_node(&mut self, _dev: &mut dyn IO, _inode: INode, _off: u64, _buf: &mut [u8])
		-> Result<usize, Errno> {
		// TODO
		todo!();
	}

	fn write_node(&mut self, _dev: &mut dyn IO, _inode: INode, _off: u64, _buf: &[u8])
		-> Result<(), Errno> {
		// TODO
		todo!();
	}
}

pub trait FilesystemType {
	fn get_name(&self) -> &str {
		"tmpfs"
	}

	fn detect(&self, _dev: &mut dyn IO) -> bool {
		false
	}

	fn create_filesystem(&self, _dev: &mut dyn IO) -> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(TmpFS::new(DEFAULT_MAX_SIZE)?)?)
	}

	fn load_filesystem(&self, dev: &mut dyn IO, _mountpath: &Path)
		-> Result<Box<dyn Filesystem>, Errno> {
		self.create_filesystem(dev)
	}
}
