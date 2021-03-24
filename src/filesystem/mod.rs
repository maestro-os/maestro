/// This module handles the filesystem hierarchy.
/// TODO doc

pub mod file_descriptor;
pub mod path;

use path::Path;

/// Type representing a user ID.
type Uid = u16;
/// Type representing a group ID.
type Gid = u16;
/// Type representing a file mode.
type Mode = u16;

/// Type representing a timestamp.
type Timestamp = u32; // TODO Move somewhere else?

/// The maximum length of a filename.
pub const MAX_NAME_LENGTH: usize = 255;

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
	name: [u8; MAX_NAME_LENGTH],
	/// The size of the file in bytes.
	size: usize,

	/// The type of the file.
	type_: FileType,

	/// The ID of the owner user.
	uid: Uid,
	/// The ID of the owner group.
	gid: Gid,
	/// The mode of the file.
	mode: Mode,

	// TODO inode

	/// Timestamp of the last modification of the inode.
	ctime: Timestamp,
	/// Timestamp of the last modification of the file.
	mtime: Timestamp,
	/// Timestamp of the last access to the file.
	atime: Timestamp,

	/// The number of hard links to the inode.
	links_count: usize,
}

impl File {
	/// Returns the file's name.
	pub fn get_name(&self) -> &str {
		"TODO" // TODO
	}

	/// Returns the size of the file in bytes.
	pub fn get_size(&self) -> usize {
		self.size
	}

	/// Returns the type of the file.
	pub fn get_file_type(&self) -> FileType {
		self.type_
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

	// TODO
}

/// Returns a reference to the file at path `path`. If the file doesn't exist, the function returns
/// None.
pub fn get_file_from_path(_path: &Path) -> Option::<&'static mut File> {
	// TODO
	None
}
