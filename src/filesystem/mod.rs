/// This module handles the filesystem hierarchy.
/// TODO doc

pub mod file_descriptor;
pub mod path;

use crate::errno::Errno;
use crate::errno;
use crate::limits;
use crate::util::container::string::String;
use path::Path;

/// Type representing a user ID.
type Uid = u16;
/// Type representing a group ID.
type Gid = u16;
/// Type representing a file mode.
type Mode = u16;

/// Type representing a timestamp.
type Timestamp = u32; // TODO Move somewhere else?

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

/// Enumeration representing the different file types.
#[derive(Copy, Clone, Debug)]
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
	inode: Option::<u32>, // TODO

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
	pub fn new(name: String, file_type: FileType, uid: Uid, gid: Gid, mode: Mode) -> Self {
		debug_assert!(name.len() <= limits::NAME_MAX);

		let timestamp = 0; // TODO
		Self {
			name: name,
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

	/// Returns the file's name.
	pub fn get_name(&self) -> &String {
		&self.name
	}

	/// Sets the file's name.
	pub fn set_name(&mut self, name: String) {
		self.name = name;
		// TODO Update to disk directly?
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

	// TODO
}

/// Adds the file `file` to the file hierarchy. The file will be located into the directory at path
/// `path`. The directory must exist. If an error happens, the function returns an Err with the
/// appropriate Errno.
pub fn add_file(_path: Path, _file: File) -> Result::<(), Errno> {
	// TODO
	Err(errno::ENOMEM)
}

/// Returns a reference to the file at path `path`. If the file doesn't exist, the function returns
/// None.
pub fn get_file_from_path(_path: &Path) -> Option::<&'static mut File> {
	// TODO
	None
}
