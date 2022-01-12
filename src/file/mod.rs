//! This module handles filesystems. Every filesystems are unified by the Virtual FileSystem (VFS).
//! The root filesystem is passed to the kernel as an argument when booting. Other filesystems are
//! mounted into subdirectories.

pub mod fcache;
pub mod file_descriptor;
pub mod fs;
pub mod mountpoint;
pub mod path;
pub mod pipe;
pub mod socket;

use core::cmp::max;
use core::ffi::c_void;
use crate::device::DeviceType;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::fcache::FCache;
use crate::file::mountpoint::MountPoint;
use crate::limits;
use crate::time::Timestamp;
use crate::time;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::ptr::SharedPtr;
use path::Path;

/// Type representing a user ID.
pub type Uid = u16;
/// Type representing a group ID.
pub type Gid = u16;
/// Type representing a file mode.
pub type Mode = u32;
/// Type representing an inode ID.
pub type INode = u32;

/// File type: socket
pub const S_IFSOCK: Mode = 0o140000;
/// File type: symbolic link
pub const S_IFLNK: Mode = 0o120000;
/// File type: regular file
pub const S_IFREG: Mode = 0o100000;
/// File type: block device
pub const S_IFBLK: Mode = 0o060000;
/// File type: directory
pub const S_IFDIR: Mode = 0o040000;
/// File type: character device
pub const S_IFCHR: Mode = 0o020000;
/// File type: FIFO
pub const S_IFIFO: Mode = 0o010000;

/// User: Read, Write and Execute.
pub const S_IRWXU: Mode = 0o0700;
/// User: Read.
pub const S_IRUSR: Mode = 0o0400;
/// User: Write.
pub const S_IWUSR: Mode = 0o0200;
/// User: Execute.
pub const S_IXUSR: Mode = 0o0100;
/// Group: Read, Write and Execute.
pub const S_IRWXG: Mode = 0o0070;
/// Group: Read.
pub const S_IRGRP: Mode = 0o0040;
/// Group: Write.
pub const S_IWGRP: Mode = 0o0020;
/// Group: Execute.
pub const S_IXGRP: Mode = 0o0010;
/// Other: Read, Write and Execute.
pub const S_IRWXO: Mode = 0o0007;
/// Other: Read.
pub const S_IROTH: Mode = 0o0004;
/// Other: Write.
pub const S_IWOTH: Mode = 0o0002;
/// Other: Execute.
pub const S_IXOTH: Mode = 0o0001;
/// Setuid.
pub const S_ISUID: Mode = 0o4000;
/// Setgid.
pub const S_ISGID: Mode = 0o2000;
/// Sticky bit.
pub const S_ISVTX: Mode = 0o1000;

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
	Fifo,
	/// A Unix domain socket.
	Socket,
	/// A Block device file.
	BlockDevice,
	/// A Character device file.
	CharDevice,
}

impl FileType {
	/// Returns the type corresponding to the given mode `mode`.
	/// If the type doesn't exist, the function returns None.
	pub fn from_mode(mode: Mode) -> Option<Self> {
		match mode & 0o770000 {
			S_IFSOCK => Some(Self::Socket),
			S_IFLNK => Some(Self::Link),
			S_IFREG | 0 => Some(Self::Regular),
			S_IFBLK => Some(Self::BlockDevice),
			S_IFDIR => Some(Self::Directory),
			S_IFCHR => Some(Self::CharDevice),
			S_IFIFO => Some(Self::Fifo),

			_ => None,
		}
	}
}

/// Structure representing the location of a file on a disk.
pub struct FileLocation {
	/// The path of the mountpoint.
	mountpoint_path: Path, // TODO Replace by an allocated ID to save memory

	/// The disk's inode.
	inode: INode,
}

impl FileLocation {
	/// Creates a new instance.
	#[inline]
	pub fn new(mountpoint_path: Path, inode: INode) -> Self {
		Self {
			mountpoint_path,

			inode,
		}
	}

	/// Returns the path of the mountpoint.
	#[inline]
	pub fn get_mountpoint_path(&self) -> &Path {
		&self.mountpoint_path
	}

	/// Returns the mountpoint associated with the file's location.
	pub fn get_mountpoint(&self) -> Option<SharedPtr<MountPoint>> {
		mountpoint::from_path(&self.mountpoint_path)
	}

	/// Returns the inode number.
	#[inline]
	pub fn get_inode(&self) -> INode {
		self.inode
	}
}

/// Enumeration of all possible file contents for each file types.
pub enum FileContent {
	/// The file is a regular file. No data.
	Regular,
	/// The file is a directory. The data is the list of subfiles.
	Directory(Vec<String>),
	/// The file is a link. The data is the link's target.
	Link(String),
	/// The file is a FIFO. The data is a pipe ID.
	Fifo(u32),
	/// The file is a socket. The data is a socket ID.
	Socket(u32),

	/// The file is a block device.
	BlockDevice {
		major: u32,
		minor: u32,
	},

	/// The file is a char device.
	CharDevice {
		major: u32,
		minor: u32,
	},
}

impl FileContent {
	/// Returns the file type associated with the content type.
	pub fn get_file_type(&self) -> FileType {
		match self {
			Self::Regular => FileType::Regular,
			Self::Directory(_) => FileType::Directory,
			Self::Link(_) => FileType::Link,
			Self::Fifo(_) => FileType::Fifo,
			Self::Socket(_) => FileType::Socket,
			Self::BlockDevice { .. } => FileType::BlockDevice,
			Self::CharDevice { .. } => FileType::CharDevice,
		}
	}
}

/// Structure representing a file.
pub struct File {
	/// The name of the file.
	name: String,

	/// The path of the file's parent.
	parent_path: Path,

	/// The size of the file in bytes.
	size: u64,

	/// The ID of the owner user.
	uid: Uid,
	/// The ID of the owner group.
	gid: Gid,
	/// The mode of the file.
	mode: Mode,

	/// The location the file is stored on.
	location: Option<FileLocation>,

	/// Timestamp of the last modification of the metadata.
	ctime: Timestamp,
	/// Timestamp of the last modification of the file.
	mtime: Timestamp,
	/// Timestamp of the last access to the file.
	atime: Timestamp,

	/// The content of the file.
	content: FileContent,
}

impl File {
	/// Creates a new instance.
	/// `name` is the name of the file.
	/// `file_content` is the content of the file. This value also determines the file type.
	/// `uid` is the id of the owner user.
	/// `gid` is the id of the owner group.
	/// `mode` is the permission of the file.
	pub fn new(name: String, content: FileContent, uid: Uid, gid: Gid, mode: Mode)
		-> Result<Self, Errno> {
		let timestamp = time::get().unwrap_or(0);

		Ok(Self {
			name,
			parent_path: Path::root(),

			size: 0,

			uid,
			gid,
			mode,

			location: None,

			ctime: timestamp,
			mtime: timestamp,
			atime: timestamp,

			content,
		})
	}

	/// Returns the name of the file.
	pub fn get_name(&self) -> &String {
		&self.name
	}

	/// Returns the absolute path of the file's parent.
	pub fn get_parent_path(&self) -> &Path {
		&self.parent_path
	}

	/// Returns the absolute path of the file.
	pub fn get_path(&self) -> Result<Path, Errno> {
		let mut parent_path = self.parent_path.failable_clone()?;
		parent_path.push(self.name.failable_clone()?)?;

		Ok(parent_path)
	}

	/// Sets the file's parent path.
	/// If the path isn't absolute, the behaviour is undefined.
	pub fn set_parent_path(&mut self, parent_path: Path) {
		self.parent_path = parent_path;
	}

	/// Sets the file's size.
	pub fn set_size(&mut self, size: u64) {
		self.size = size;
	}

	/// Returns the type of the file.
	pub fn get_file_type(&self) -> FileType {
		self.content.get_file_type()
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

	/// Tells if the file can be read from by the given UID and GID.
	pub fn can_read(&self, uid: Uid, gid: Gid) -> bool {
		// If root, bypass checks
		if uid == 0 || gid == 0 {
			return true;
		}

		if self.mode & S_IRUSR != 0 && self.uid == uid {
			return true;
		}
		if self.mode & S_IRGRP != 0 && self.gid == gid {
			return true;
		}
		self.mode & S_IROTH != 0
	}

	/// Tells if the file can be written to by the given UID and GID.
	pub fn can_write(&self, uid: Uid, gid: Gid) -> bool {
		// If root, bypass checks
		if uid == 0 || gid == 0 {
			return true;
		}

		if self.mode & S_IWUSR != 0 && self.uid == uid {
			return true;
		}
		if self.mode & S_IWGRP != 0 && self.gid == gid {
			return true;
		}
		self.mode & S_IWOTH != 0
	}

	/// Tells if the file can be executed by the given UID and GID.
	pub fn can_execute(&self, uid: Uid, gid: Gid) -> bool {
		// If root, bypass checks
		if uid == 0 || gid == 0 {
			return true;
		}

		if self.mode & S_IXUSR != 0 && self.uid == uid {
			return true;
		}
		if self.mode & S_IXGRP != 0 && self.gid == gid {
			return true;
		}
		self.mode & S_IXOTH != 0
	}

	/// Returns the location on which the file is stored.
	pub fn get_location(&self) -> &Option<FileLocation> {
		&self.location
	}

	/// Sets the location on which the file is stored.
	pub fn set_location(&mut self, location: Option<FileLocation>) {
		self.location = location;
	}

	/// Returns the timestamp of the last modification of the file's metadata.
	pub fn get_ctime(&self) -> Timestamp {
		self.ctime
	}

	/// Sets the timestamp of the last modification of the file's metadata.
	pub fn set_ctime(&mut self, ctime: Timestamp) {
		self.ctime = ctime;
	}

	/// Returns the timestamp of the last modification to the file.
	pub fn get_mtime(&self) -> Timestamp {
		self.mtime
	}

	/// Sets the timestamp of the last modification to the file.
	pub fn set_mtime(&mut self, mtime: Timestamp) {
		self.mtime = mtime;
	}

	/// Returns the timestamp of the last access to the file.
	pub fn get_atime(&self) -> Timestamp {
		self.atime
	}

	/// Sets the timestamp of the last access to the file.
	pub fn set_atime(&mut self, atime: Timestamp) {
		self.atime = atime;
	}

	/// Tells whether the directory is empty or not. If the current file is not a directory, the
	/// behaviour is undefined.
	pub fn is_empty_directory(&self) -> bool {
		if let FileContent::Directory(subfiles) = &self.content {
			subfiles.is_empty()
		} else {
			panic!("Not a directory!");
		}
	}

	/// Adds the file with name `name` to the current file's subfiles. If the current file isn't a
	/// directory, the behaviour is undefined.
	pub fn add_subfile(&mut self, name: String) -> Result<(), Errno> {
		if let FileContent::Directory(subfiles) = &mut self.content {
			subfiles.push(name)
		} else {
			panic!("Not a directory!");
		}
	}

	/// Removes the file with name `name` from the current file's subfiles. If the current file
	/// isn't a directory, the behaviour is undefined.
	pub fn remove_subfile(&mut self, _name: String) {
		if let FileContent::Directory(_subfiles) = &mut self.content {
			// TODO
			todo!();
			// subfiles.remove(name);
		} else {
			panic!("Not a directory!");
		}
	}

	/// Returns the file's content.
	pub fn get_file_content(&self) -> &FileContent {
		&self.content
	}

	/// Sets the file's content, changing the file's type accordingly.
	pub fn set_file_content(&mut self, content: FileContent) {
		self.content = content;
	}

	/// Performs an ioctl operation on the file.
	pub fn ioctl(&mut self, request: u32, argp: *const c_void) -> Result<u32, Errno> {
		if let FileContent::CharDevice {
			major,
			minor,
		} = self.content {
			let dev = device::get_device(DeviceType::Char, major, minor).ok_or(errno::ENODEV)?;
			let mut guard = dev.lock();
			guard.get_mut().get_handle().ioctl(request, argp)
		} else {
			Err(errno::ENOTTY)
		}
	}

	/// Synchronizes the file with the device.
	pub fn sync(&self) {
		// TODO
	}

	/// Unlinks the current file.
	pub fn unlink(&mut self) {
		// TODO
	}
}

impl IO for File {
	fn get_size(&self) -> u64 {
		self.size
	}

	fn read(&self, off: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		match &self.content {
			FileContent::Regular => {
				let location = self.location.as_ref().ok_or(errno::EIO)?;

				let mountpoint_mutex = location.get_mountpoint().ok_or(errno::EIO)?;
				let mut mountpoint_guard = mountpoint_mutex.lock();
				let mountpoint = mountpoint_guard.get_mut();

				let io_mutex = mountpoint.get_source().get_io().clone();
				let mut io_guard = io_mutex.lock();
				let io = io_guard.get_mut();

				let filesystem = mountpoint.get_filesystem();
				filesystem.read_node(io, location.get_inode(), off, buff)
			},

			FileContent::Directory(_subdirs) => {
				// TODO
				todo!();
			},

			FileContent::Link(_target) => {
				// TODO
				todo!();
			},

			FileContent::Fifo(_pipe) => {
				// TODO
				todo!();
			},

			FileContent::Socket(_socket) => {
				// TODO
				todo!();
			},

			FileContent::BlockDevice { .. } | FileContent::CharDevice { .. } => {
				let dev = match self.content {
					FileContent::BlockDevice { major, minor } => {
						device::get_device(DeviceType::Block, major, minor)
					},

					FileContent::CharDevice { major, minor } => {
						device::get_device(DeviceType::Char, major, minor)
					},

					_ => unreachable!(),
				}.ok_or(errno::ENODEV)?;

				let mut guard = dev.lock();
				guard.get_mut().get_handle().read(off as _, buff)
			},
		}
	}

	fn write(&mut self, off: u64, buff: &[u8]) -> Result<usize, Errno> {
		match &self.content {
			FileContent::Regular => {
				let location = self.location.as_ref().ok_or(errno::EIO)?;

				let mountpoint_mutex = location.get_mountpoint().ok_or(errno::EIO)?;
				let mut mountpoint_guard = mountpoint_mutex.lock();
				let mountpoint = mountpoint_guard.get_mut();

				let io_mutex = mountpoint.get_source().get_io();
				let mut io_guard = io_mutex.lock();
				let io = io_guard.get_mut();

				let filesystem = mountpoint.get_filesystem();
				filesystem.write_node(io, location.get_inode(), off, buff)?;

				self.size = max(off + buff.len() as u64, self.size);
				Ok(buff.len())
			},

			FileContent::Directory(_subdirs) => {
				// TODO
				todo!();
			},

			FileContent::Link(_target) => {
				// TODO
				todo!();
			},

			FileContent::Fifo(_pipe) => {
				// TODO
				todo!();
			},

			FileContent::Socket(_socket) => {
				// TODO
				todo!();
			},

			FileContent::BlockDevice { .. } | FileContent::CharDevice { .. } => {
				let dev = match self.content {
					FileContent::BlockDevice { major, minor } => {
						device::get_device(DeviceType::Block, major, minor)
					},

					FileContent::CharDevice { major, minor } => {
						device::get_device(DeviceType::Char, major, minor)
					},

					_ => unreachable!(),
				}.ok_or(errno::ENODEV)?;

				let mut guard = dev.lock();
				guard.get_mut().get_handle().write(off as _, buff)
			},
		}
	}
}

impl Drop for File {
	fn drop(&mut self) {
		self.sync();
	}
}

/// Resolves symbolic links and returns the final path. If too many links are to be resolved, the
/// function returns an error.
/// `file` is the starting file. If not a link, the function returns the path to this file.
/// If the file pointed by the link(s) doesn't exist, the function returns the path where the file
/// should be located.
pub fn resolve_links(file: SharedPtr<File>) -> Result<Path, Errno> {
	let mut resolve_count = 0;
	let mut file = file;

	// Resolve links until the current file is not a link
	while resolve_count <= limits::SYMLOOP_MAX {
		let file_guard = file.lock();
		let f = file_guard.get();

		// Get the path of the parent directory of the current file
		let parent_path = f.get_parent_path();

		// If the file is a link, resolve it. Else, break the loop
		if let FileContent::Link(link_target) = f.get_file_content() {
			// Resolving the link
			let link_path = Path::from_str(link_target.as_bytes(), false)?;
			let mut path = (parent_path.failable_clone()? + link_path)?;
			path.reduce()?;
			drop(file_guard);

			// Getting the file from path
			let mutex = fcache::get();
			let mut guard = mutex.lock();
			let files_cache = guard.get_mut().as_mut().unwrap();

			match files_cache.get_file_from_path(&path) {
				Ok(next_file) => file = next_file,
				Err(e) => return {
					if e == errno::ENOENT {
						Ok(path)
					} else {
						Err(e)
					}
				},
			}
		} else {
			break;
		}

		resolve_count += 1;
	}

	if resolve_count <= limits::SYMLOOP_MAX {
		let file_guard = file.lock();
		let f = file_guard.get();

		f.get_path()
	} else {
		Err(errno::ELOOP)
	}
}

/// Initializes files management.
/// `root_device_type` is the type of the root device file. If not a device, the behaviour is
/// undefined.
/// `root_major` is the major number of the device at the root of the VFS.
/// `root_minor` is the minor number of the device at the root of the VFS.
pub fn init(root_device_type: DeviceType, root_major: u32, root_minor: u32) -> Result<(), Errno> {
	fs::register_defaults()?;

	// The root device
	let root_dev = device::get_device(root_device_type, root_major, root_minor)
		.ok_or(errno::ENODEV)?;

	// Creating the files cache
	let cache = FCache::new(root_dev)?;
	let mut guard = fcache::get().lock();
	*guard.get_mut() = Some(cache);

	Ok(())
}
