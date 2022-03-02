//! Tmpfs (Temporary file system) is, as its name states a temporary filesystem. The files are
//! stored on the kernel's memory and thus are removed when the filesystem is unmounted.

use core::cmp::min;
use core::mem::size_of;
use crate::errno;
use crate::file::Errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::Filesystem;
use crate::file::fs::FilesystemType;
use crate::file::fs::kernfs::KernFS;
use crate::file::fs::kernfs::KernFSNode;
use crate::file::inode::INode;
use crate::file::path::Path;
use crate::time::Timestamp;
use crate::time;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// The default maximum amount of memory the filesystem can use in bytes.
const DEFAULT_MAX_SIZE: usize = 512 * 1024 * 1024;

/// Structure representing a file in a tmpfs.
pub struct TmpFSFile {
	/// The file's content.
	content: FileContent,

	/// The content of the file, if it is a regular file.
	regular_content: Vec<u8>, // TODO Only if the file is regular
	/// The content of the file, if it is a directory.
	entries: HashMap<String, Box<dyn KernFSNode>>, // TODO Only if the file is a directory

	/// The file's permissions.
	mode: Mode,
	/// The file's user ID.
	uid: Uid,
	/// The file's group ID.
	gid: Gid,

	/// TODO doc
	atime: Timestamp,
	/// TODO doc
	ctime: Timestamp,
	/// TODO doc
	mtime: Timestamp,
}

impl TmpFSFile {
	/// Creates a new instance.
	/// `content` is the file's content.
	/// `mode` is the file's permissions.
	/// `uid` is the file owner's uid.
	/// `gid` is the file owner's gid.
	/// `ts` is the current timestamp.
	/// If the file is a directory, the given list of entries is ignored since they cannot be
	/// associated to any inode immediately.
	pub fn new(content: FileContent, mode: Mode, uid: Uid, gid: Gid, ts: Timestamp) -> Self {
		Self {
			content,

			regular_content: Vec::new(),
			entries: HashMap::new(),

			mode,
			uid,
			gid,

			atime: ts,
			ctime: ts,
			mtime: ts,
		}
	}

	/// Returns the size used by the file in bytes.
	pub fn get_used_size(&self) -> usize {
		size_of::<Self>() + self.get_size() as usize
	}
}

impl KernFSNode for TmpFSFile {
	fn get_type(&self) -> FileType {
		self.content.get_file_type()
	}

	fn get_mode(&self) -> Mode {
		self.mode
	}

	fn set_mode(&mut self, mode: Mode) {
		self.mode = mode;
	}

	fn get_uid(&self) -> Uid {
		self.uid
	}

	fn set_uid(&mut self, uid: Uid) {
		self.uid = uid;
	}

	fn get_gid(&self) -> Gid {
		self.gid
	}

	fn set_gid(&mut self, gid: Gid) {
		self.gid = gid;
	}

	fn get_atime(&self) -> Timestamp {
		self.atime
	}

	fn set_atime(&mut self, ts: Timestamp) {
		self.atime = ts;
	}

	fn get_ctime(&self) -> Timestamp {
		self.ctime
	}

	fn set_ctime(&mut self, ts: Timestamp) {
		self.ctime = ts;
	}

	fn get_mtime(&self) -> Timestamp {
		self.mtime
	}

	fn set_mtime(&mut self, ts: Timestamp) {
		self.mtime = ts;
	}

	fn get_entries(&self) -> Result<HashMap<String, Box<dyn KernFSNode>>, Errno> {
		Ok(self.entries)
	}
}

impl IO for TmpFSFile {
	fn get_size(&self) -> u64 {
		match &self.content {
			FileContent::Regular => self.regular_content.len() as _,
			FileContent::Directory(_) => {
				let names_len = self.entries.iter()
					.map(| (name, _) | name.len() as u64)
					.sum::<u64>();

				// Adding the length of inodes
				names_len + (self.entries.len() as u64 * size_of::<TmpFSFile>() as u64)
			},
			FileContent::Link(path) => path.len() as _,
			FileContent::Fifo => 0, // TODO Add the size of the id?
			FileContent::Socket => 0, // TODO Add the size of the id?
			FileContent::BlockDevice { .. } | FileContent::CharDevice { .. } => 0,
		}
	}

	fn read(&self, offset: u64, buff: &mut [u8]) -> Result<u64, Errno> {
		// TODO Avoid redundant code with casual files
		match &self.content {
			FileContent::Regular => {
				if offset <= self.regular_content.len() as _ {
					buff.copy_from_slice(&self.regular_content.as_slice()[(offset as usize)..]);
					Ok(min(buff.len() as u64, self.regular_content.len() as u64 - offset))
				} else {
					Ok(0)
				}
			},

			FileContent::Directory(_) => Err(errno!(EINVAL)),

			FileContent::Link(_) => Err(errno!(EINVAL)),

			FileContent::Fifo => {
				// TODO
				todo!();
			},

			FileContent::Socket => {
				// TODO
				todo!();
			},

			FileContent::BlockDevice {
				major: _,
				minor: _
			} => {
				// TODO
				todo!();
			},

			FileContent::CharDevice {
				major: _,
				minor: _
			} => {
				// TODO
				todo!();
			},
		}
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		// TODO Avoid redundant code with casual files
		match &self.content {
			FileContent::Regular => {
				if offset <= self.regular_content.len() as u64 {
					if offset + buff.len() as u64 <= usize::MAX as u64 {
						// Increase the size of storage if necessary
						if offset + buff.len() as u64 > self.regular_content.len() as u64 {
							self.regular_content.resize(offset as usize + buff.len())?;
						}

						self.regular_content.as_mut_slice()[(offset as usize)..]
							.copy_from_slice(&buff);
						Ok(buff.len() as _)
					} else {
						Err(errno!(EFBIG))
					}
				} else {
					Err(errno!(EINVAL))
				}
			},

			FileContent::Directory(_) => Err(errno!(EINVAL)),

			FileContent::Link(_) => Err(errno!(EINVAL)),

			FileContent::Fifo => {
				// TODO
				todo!();
			},

			FileContent::Socket => {
				// TODO
				todo!();
			},

			FileContent::BlockDevice {
				major: _,
				minor: _
			} => {
				// TODO
				todo!();
			},

			FileContent::CharDevice {
				major: _,
				minor: _
			} => {
				// TODO
				todo!();
			},
		}
	}
}

/// Structure representing the temporary file system.
/// On the inside, the tmpfs works using a kernfs.
pub struct TmpFS {
	/// The maximum amount of memory in bytes the filesystem can use.
	max_size: usize,
	/// The currently used amount of memory in bytes.
	size: usize,

	/// The kernfs.
	fs: KernFS,
}

impl TmpFS {
	/// Creates a new instance.
	/// `max_size` is the maximum amount of memory the filesystem can use in bytes.
	/// `readonly` tells whether the filesystem is readonly.
	pub fn new(max_size: usize, readonly: bool) -> Result<Self, Errno> {
		let mut fs = Self {
			max_size,
			size: 0,

			fs: KernFS::new(String::from(b"tmpfs")?, readonly),
		};

		// The current timestamp
		let ts = time::get().unwrap_or(0);

		// Adding the root node
		let root_node = TmpFSFile::new(FileContent::Directory(crate::vec![]), 0o777, 0, 0, ts);
		fs.update_size(root_node.get_used_size() as _, | fs | {
			fs.fs.set_root(Some(Box::new(root_node)?));
			Ok(())
		})?;

		Ok(fs)
	}

	/// Executes the given function `f`. On success, the function adds `s` to the total size of the
	/// filesystem.
	/// If `f` fails, the function doesn't change the total size and returns the error.
	/// If the new total size is too large, `f` is not executed and the function returns an error.
	fn update_size<F: FnOnce(&mut Self) -> Result<(), Errno>>(&mut self, s: isize, f: F)
		-> Result<(), Errno> {
		if s < 0 {
			f(self)?;

			if self.size < (-s as usize) {
				// If the result would underflow, set the total to zero
				self.size = 0;
			} else {
				self.size -= -s as usize;
			}

			Ok(())
		} else {
			if self.size + (s as usize) < self.max_size {
				f(self)?;

				self.size += s as usize;
				Ok(())
			} else {
				Err(errno!(ENOSPC))
			}
		}
	}
}

impl Filesystem for TmpFS {
	fn get_name(&self) -> &[u8] {
		self.fs.get_name()
	}

	fn is_readonly(&self) -> bool {
		self.fs.is_readonly()
	}

	fn must_cache(&self) -> bool {
		self.fs.must_cache()
	}

	fn get_inode(&mut self, io: &mut dyn IO, parent: Option<Box<dyn INode>>, name: Option<&String>)
		-> Result<Box<dyn INode>, Errno> {
		self.fs.get_inode(io, parent, name)
	}

	fn load_file(&mut self, io: &mut dyn IO, inode: Box<dyn INode>, name: String)
		-> Result<File, Errno> {
		self.fs.load_file(io, inode, name)
	}

	fn add_file(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn INode>, _name: String,
		_uid: Uid, _gid: Gid, _mode: Mode, _content: FileContent) -> Result<File, Errno> {
		// TODO
		todo!();
	}

	fn add_link(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn INode>, _name: &String,
		_inode: Box<dyn INode>) -> Result<(), Errno> {
		// TODO
		todo!();
	}

	fn update_inode(&mut self, _io: &mut dyn IO, _file: &File) -> Result<(), Errno> {
		// TODO
		todo!();
	}

	fn remove_file(&mut self, _io: &mut dyn IO, _parent_inode: Box<dyn INode>, _name: &String)
		-> Result<(), Errno> {
		// TODO
		todo!();
	}

	fn read_node(&mut self, io: &mut dyn IO, inode: Box<dyn INode>, off: u64, buf: &mut [u8])
		-> Result<u64, Errno> {
		self.fs.read_node(io, inode, off, buf)
	}

	fn write_node(&mut self, _io: &mut dyn IO, _inode: Box<dyn INode>, _off: u64, _buf: &[u8])
		-> Result<(), Errno> {
		// TODO
		todo!();
	}
}

/// Structure representing the tmpfs file system type.
pub struct TmpFsType {}

impl FilesystemType for TmpFsType {
	fn get_name(&self) -> &[u8] {
		b"tmpfs"
	}

	fn detect(&self, _io: &mut dyn IO) -> Result<bool, Errno> {
		Ok(false)
	}

	fn create_filesystem(&self, _io: &mut dyn IO) -> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(TmpFS::new(DEFAULT_MAX_SIZE, false)?)?)
	}

	fn load_filesystem(&self, _io: &mut dyn IO, _mountpath: Path, readonly: bool)
		-> Result<Box<dyn Filesystem>, Errno> {
		Ok(Box::new(TmpFS::new(DEFAULT_MAX_SIZE, readonly)?)?)
	}
}
