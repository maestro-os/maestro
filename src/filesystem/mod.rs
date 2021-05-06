/// This module handles filesystems. Every filesystems are unified by the Virtual FileSystem (VFS).
/// The root filesystem is passed to the kernel as an argument when booting. Other filesystems are
/// mounted into subdirectories.

pub mod file_descriptor;
pub mod filesystem;
pub mod mountpoint;
pub mod path;

use crate::errno::Errno;
use crate::errno;
use crate::time::Timestamp;
use crate::time;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::ptr::WeakPtr;
use path::Path;

/// Type representing a user ID.
pub type Uid = u16;
/// Type representing a group ID.
pub type Gid = u16;
/// Type representing a file mode.
pub type Mode = u16;
/// Type representing an inode ID.
pub type INode = u32;

/// TODO doc
pub const S_IRWXU: Mode = 00700;
/// TODO doc
pub const S_IRUSR: Mode = 00400;
/// TODO doc
pub const S_IWUSR: Mode = 00200;
/// TODO doc
pub const S_IXUSR: Mode = 00100;
/// TODO doc
pub const S_IRWXG: Mode = 00070;
/// TODO doc
pub const S_IRGRP: Mode = 00040;
/// TODO doc
pub const S_IWGRP: Mode = 00020;
/// TODO doc
pub const S_IXGRP: Mode = 00010;
/// TODO doc
pub const S_IRWXO: Mode = 00007;
/// TODO doc
pub const S_IROTH: Mode = 00004;
/// TODO doc
pub const S_IWOTH: Mode = 00002;
/// TODO doc
pub const S_IXOTH: Mode = 00001;
/// TODO doc
pub const S_ISUID: Mode = 04000;
/// TODO doc
pub const S_ISGID: Mode = 02000;
/// TODO doc
pub const S_ISVTX: Mode = 01000;

/// The size of the files pool.
pub const FILES_POOL_SIZE: usize = 1024;

/// Enumeration representing the different file types.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum FileType {
	/// A regular file storing data.
	Regular,
	/// A directory, containing other files.
	Directory,
	/// A symbolic link, pointing to another file.
	Link,
	/// A named pipe.
	FIFO,
	/// A Unix domain socket.
	Socket,
	/// A Block device file.
	BlockDevice,
	/// A Character device file.
	CharDevice,
}

/// Structure representing a file.
pub struct File {
	/// The name of the file.
	name: String,

	/// Pointer to the parent file.
	parent: WeakPtr<File>,

	/// The size of the file in bytes.
	size: usize,
	/// The type of the file.
	file_type: FileType,

	/// The ID of the owner user.
	uid: Uid,
	/// The ID of the owner group.
	gid: Gid,
	/// The mode of the file.
	mode: Mode,

	/// The inode. None means that the file is not stored on any filesystem.
	inode: Option::<INode>,

	/// Timestamp of the last modification of the metadata.
	ctime: Timestamp,
	/// Timestamp of the last modification of the file.
	mtime: Timestamp,
	/// Timestamp of the last access to the file.
	atime: Timestamp,

	// TODO Store file data:
	// - Regular: text
	// - Directory: children files
	// - Link: target
	// - FIFO: buffer (on ram only)
	// - Socket: buffer (on ram only)
	// - BlockDevice: major and minor
	// - CharDevice: major and minor
}

impl File {
	/// Creates a new instance.
	pub fn new(name: String, parent: WeakPtr<File>, file_type: FileType, uid: Uid, gid: Gid,
		mode: Mode) -> Self {
		let timestamp = time::get();

		Self {
			name: name,
			parent: parent,

			size: 0,
			file_type: file_type,

			uid: uid,
			gid: gid,
			mode: mode,

			inode: None,

			ctime: timestamp,
			mtime: timestamp,
			atime: timestamp,
		}
	}

	/// Returns the size of the file in bytes.
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Returns the type of the file.
	pub fn get_file_type(&self) -> FileType {
		self.file_type
	}

	/// Returns the owner user ID.
	pub fn get_uid(&self) -> Uid {
		self.uid
	}

	/// Returns the owner group ID.
	pub fn get_gid(&self) -> Gid {
		self.gid
	}

	/// Returns the file's mode.
	pub fn get_mode(&self) -> Mode {
		self.mode
	}

	/// Returns the timestamp to the last modification of the file's metadata.
	pub fn get_ctime(&self) -> Timestamp {
		self.ctime
	}

	/// Returns the timestamp to the last modification to the file.
	pub fn get_mtime(&self) -> Timestamp {
		self.mtime
	}

	/// Returns the timestamp to the last access to the file.
	pub fn get_atime(&self) -> Timestamp {
		self.atime
	}

	/// Tells if the file can be read from by the given UID and GID.
	pub fn can_read(&self, uid: Uid, gid: Gid) -> bool {
		if self.uid == uid && self.mode & S_IRUSR != 0 {
			return true;
		}
		if self.gid == gid && self.mode & S_IRGRP != 0 {
			return true;
		}
		self.mode & S_IROTH != 0
	}

	/// Tells if the file can be written to by the given UID and GID.
	pub fn can_write(&self, uid: Uid, gid: Gid) -> bool {
		if self.uid == uid && self.mode & S_IWUSR != 0 {
			return true;
		}
		if self.gid == gid && self.mode & S_IWGRP != 0 {
			return true;
		}
		self.mode & S_IWOTH != 0
	}

	/// Tells if the file can be executed by the given UID and GID.
	pub fn can_execute(&self, uid: Uid, gid: Gid) -> bool {
		if self.uid == uid && self.mode & S_IXUSR != 0 {
			return true;
		}
		if self.gid == gid && self.mode & S_IXGRP != 0 {
			return true;
		}
		self.mode & S_IXOTH != 0
	}

	/// Synchronizes the file's content with the device.
	pub fn sync(&self) {
		if self.inode.is_some() {
			// TODO
		}
	}

	/// Unlinks the current file.
	pub fn unlink(&mut self) {
		// TODO
	}
}

///	Cache storing files in memory. This cache allows to speedup accesses to the disk. It is
/// synchronized with the disk when necessary.
pub struct FCache {
	/// The major number of the root device.
	root_major: u32,
	/// The minor number of the root device.
	root_minor: u32,

	/// A fixed-size pool storing files, sorted by approximated number of accesses count.
	pool: Vec<File>,
}

impl FCache {
	/// Creates a new instance with the given major and minor for the root device.
	pub fn new(root_major: u32, root_minor: u32) -> Result<Self, Errno> {
		Ok(Self {
			root_major: root_major,
			root_minor: root_minor,

			pool: Vec::<File>::with_capacity(FILES_POOL_SIZE)?,
		})
	}

	// TODO
}

/// Adds the file `file` to the VFS. The file will be located into the directory at path `path`.
/// The directory must exist. If an error happens, the function returns an Err with the appropriate
/// Errno.
/// If the path is relative, the function starts from the root.
pub fn create_file(_path: &Path, _file: File) -> Result<(), Errno> {
	// TODO
	Err(errno::ENOMEM)
}

/// Returns a reference to the file at path `path`. If the file doesn't exist, the function returns
/// None.
/// If the path is relative, the function starts from the root.
pub fn get_file_from_path(_path: &Path) -> Option<&'static mut File> {
	// TODO
	None
}

/// Creates directories recursively on path `path`. On success, the function returns the deepest
/// directory that has been created.
/// If the directories already exist, the function does nothing.
pub fn create_dirs(_path: &Path) -> Result<WeakPtr<File>, Errno> {
	// TODO
	Err(errno::ENOMEM)
}
