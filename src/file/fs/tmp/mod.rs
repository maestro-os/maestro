//! Tmpfs (Temporary file system) is, as its name states a temporary filesystem. The files are
//! stored on the kernel's memory and thus are removed when the filesystem is unmounted.

use crate::file::Errno;
use crate::file::File;
use crate::file::INode;
use crate::file::fs::Device;
use crate::file::fs::Filesystem;
use crate::file::path::Path;
use crate::util::boxed::Box;
use crate::util::container::string::String;

// TODO Take as parameter when mounting
/// The maximum amount of memory the filesystem can use in bytes.
const TMPFS_MAX_SIZE: usize = 512 * 1024 * 1024;

// TODO Use structure `File` directly instead?
/// Structure representing a file stored in a tmpfs.
pub struct TmpfsFile {
	// TODO
}

/// Structure representing the temporary file system.
pub struct Tmpfs {
	/// The maximum amount of memory in bytes the filesystem can use.
	max_size: usize,
	/// The currently used amount of memory in bytes.
	size: usize,

	/// The files, ordered by inode number.
	files: Vec<TmpfsFile>,
}

impl Tmpfs {
	/// Creates a new instance.
	/// `max_size` is the maximum amount of memory the filesystem can use in bytes.
	pub fn new(max_size: usize) -> Self {
		Self {
			max_size,
			size: 0,

			files: Vec::new(),
		}
	}
}

impl Filesystem for Tmpfs {
	fn get_name(&self) -> &str {
		"tmpfs"
	}

	fn is_readonly(&self) -> bool {
		false
	}
	fn must_cache(&self) -> bool {
		false
	}

	fn get_inode(&mut self, _dev: &mut Device, _path: Path) -> Result<INode, Errno> {
		// TODO
		todo!();
	}

	fn load_file(&mut self, _dev: &mut Device, _inode: INode, _name: String)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn add_file(&mut self, _dev: &mut Device, _parent_inode: INode, _file: File)
		-> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn remove_file(&mut self, _dev: &mut Device, _parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		// TODO
		todo!();
	}

	fn read_node(&mut self, _dev: &mut Device, _inode: INode, _off: u64, _buf: &mut [u8])
		-> Result<usize, Errno> {
		// TODO
		todo!();
	}

	fn write_node(&mut self, _dev: &mut Device, _inode: INode, _off: u64, _buf: &[u8])
		-> Result<(), Errno> {
		// TODO
		todo!();
	}
}

pub trait FilesystemType {
	fn get_name(&self) -> &str {
		"tmpfs"
	}

	fn detect(&self, _dev: &mut Device) -> bool {
		false
	}

	fn create_filesystem(&self, _dev: &mut Device) -> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(Tmpfs::new(TMPFS_MAX_SIZE))?)
	}

	fn load_filesystem(&self, dev: &mut Device, _mountpath: &Path)
		-> Result<Box<dyn Filesystem>, Errno> {
		self.create_filesystem(dev)
	}
}
