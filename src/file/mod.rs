//! This module handles filesystems. Every filesystems are unified by the Virtual FileSystem (VFS).
//! The root filesystem is passed to the kernel as an argument when booting. Other filesystems are
//! mounted into subdirectories.

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
use crate::file::mountpoint::MountPoint;
use crate::time::Timestamp;
use crate::time;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::mutex::Mutex;
use crate::util::ptr::SharedPtr;
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

/// The size of the files pool.
pub const FILES_POOL_SIZE: usize = 1024;
/// The upper bount for the file accesses counter.
pub const ACCESSES_UPPER_BOUND: usize = 128;

// TODO Check files/directories access permissions when getting, creating, removing, etc...
// TODO Implement EROFS

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

/// Structure representing the location of a file on a disk.
#[derive(Clone, Debug)]
pub struct DiskLocation {
	/// The type of the device.
	device_type: DeviceType,
	/// The disk's major number.
	major: u32,
	/// The disk's minor number.
	minor: u32,

	/// The disk's inode.
	inode: INode,
}

impl DiskLocation {
	/// Creates a new instance.
	#[inline]
	pub fn new(device_type: DeviceType, major: u32, minor: u32, inode: INode) -> Self {
		Self {
			device_type,
			major,
			minor,

			inode,
		}
	}

	/// Returns the device type.
	#[inline]
	pub fn get_device_type(&self) -> DeviceType {
		self.device_type
	}

	/// Returns the major number.
	#[inline]
	pub fn get_major(&self) -> u32 {
		self.major
	}

	/// Returns the minor number.
	#[inline]
	pub fn get_minor(&self) -> u32 {
		self.minor
	}

	/// Returns the inode number.
	#[inline]
	pub fn get_inode(&self) -> INode {
		self.inode
	}
}

/// Enumeration of all possible locations for a file.
#[derive(Clone, Debug)]
pub enum FileLocation {
	/// The file is stored nowhere.
	None,
	/// The file is stored on a disk.
	Disk(DiskLocation),
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
	/// The file is a block device. The data is a major and minor number.
	BlockDevice(u32, u32),
	/// The file is a char device. The data is a major and minor number.
	CharDevice(u32, u32),
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
			Self::BlockDevice(_, _) => FileType::BlockDevice,
			Self::CharDevice(_, _) => FileType::CharDevice,
		}
	}
}

/// Structure representing a file.
pub struct File {
	/// The name of the file.
	name: String,

	/// Pointer to the parent file.
	parent: Option<WeakPtr<File>>,

	/// The size of the file in bytes.
	size: u64,

	/// The ID of the owner user.
	uid: Uid,
	/// The ID of the owner group.
	gid: Gid,
	/// The mode of the file.
	mode: Mode,

	/// The location the file is stored on.
	location: FileLocation,

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
		let timestamp = time::get();

		Ok(Self {
			name,
			parent: None,

			size: 0,

			uid,
			gid,
			mode,

			location: FileLocation::None,

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

	/// Returns a reference to the parent file.
	pub fn get_parent(&self) -> Option<&mut Mutex<File>> {
		self.parent.as_ref()?.get_mut()
	}

	// FIXME: Potential deadlock when locking parent
	/// Returns the absolute path of the file.
	pub fn get_path(&self) -> Result<Path, Errno> {
		let name = self.get_name().failable_clone()?;

		if let Some(parent) = self.get_parent() {
			let mut path = parent.lock(true).get().get_path()?;
			path.push(name)?;
			Ok(path)
		} else {
			let mut path = Path::root();
			path.push(name)?;
			Ok(path)
		}
	}

	/// Sets the file's parent.
	pub fn set_parent(&mut self, parent: Option<WeakPtr<File>>) {
		self.parent = parent;
	}

	/// Returns the size of the file in bytes.
	pub fn get_size(&self) -> u64 {
		self.size
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
	pub fn get_location(&self) -> &FileLocation {
		&self.location
	}

	/// Sets the location on which the file is stored.
	pub fn set_location(&mut self, location: FileLocation) {
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

	/// Reads from the current file at offset `off` and places the data into the buffer `buff`.
	/// The function returns the number of characters read.
	pub fn read(&self, off: u64, buff: &mut [u8]) -> Result<usize, Errno> {
		match &self.content {
			FileContent::Regular => {
				if let FileLocation::Disk(location) = &self.location {
					let mountpoint_mutex = mountpoint::get_from_device(
						location.get_device_type(),
						location.get_major(),
						location.get_minor()).unwrap(); // TODO Check unwrap
					let mut mountpoint_guard = mountpoint_mutex.lock(true);
					let mountpoint = mountpoint_guard.get_mut();

					let device_mutex = mountpoint.get_device().clone();
					let mut device_guard = device_mutex.lock(true);
					let device = device_guard.get_mut();

					let filesystem = mountpoint.get_filesystem();
					filesystem.read_node(device, location.get_inode(), off, buff)
				} else {
					// TODO Read from memory? Panic?
					todo!();
				}
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

			FileContent::BlockDevice(_, _) | FileContent::CharDevice(_, _) => {
				let dev = match self.content {
					FileContent::BlockDevice(major, minor) => {
						device::get_device(DeviceType::Block, major, minor)
					},
					FileContent::CharDevice(major, minor) => {
						device::get_device(DeviceType::Char, major, minor)
					},

					_ => unreachable!(),
				}.ok_or(errno::ENODEV)?;

				let mut guard = dev.lock(true);
				guard.get_mut().get_handle().read(off as _, buff)
			},
		}
	}

	/// Writes to the current file at offset `off`, reading the data from the buffer `buff`.
	/// The function returns the number of characters written.
	pub fn write(&mut self, off: u64, buff: &[u8]) -> Result<usize, Errno> {
		match &self.content {
			FileContent::Regular => {
				if let FileLocation::Disk(location) = &self.location {
					let mountpoint_mutex = mountpoint::get_from_device(
						location.get_device_type(),
						location.get_major(),
						location.get_minor()).unwrap(); // TODO Check unwrap
					let mut mountpoint_guard = mountpoint_mutex.lock(true);
					let mountpoint = mountpoint_guard.get_mut();

					let device_mutex = mountpoint.get_device().clone();
					let mut device_guard = device_mutex.lock(true);
					let device = device_guard.get_mut();

					let filesystem = mountpoint.get_filesystem();
					filesystem.write_node(device, location.get_inode(), off, buff)?;
					self.size = max(buff.len() as u64, self.size);
					Ok(buff.len())
				} else {
					// TODO Write to memory? Panic?
					todo!();
				}
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

			FileContent::BlockDevice(_, _) | FileContent::CharDevice(_, _) => {
				let dev = match self.content {
					FileContent::BlockDevice(major, minor) => {
						device::get_device(DeviceType::Block, major, minor)
					},
					FileContent::CharDevice(major, minor) => {
						device::get_device(DeviceType::Char, major, minor)
					},

					_ => unreachable!(),
				}.ok_or(errno::ENODEV)?;

				let mut guard = dev.lock(true);
				guard.get_mut().get_handle().write(off as _, buff)
			},
		}
	}

	/// Performs an ioctl operation on the file.
	pub fn ioctl(&mut self, request: u32, argp: *const c_void) -> Result<u32, Errno> {
		if let FileContent::CharDevice(major, minor) = self.content {
			let dev = device::get_device(DeviceType::Char, major, minor).ok_or(errno::ENODEV)?;
			let mut guard = dev.lock(true);
			guard.get_mut().get_handle().ioctl(request, argp)
		} else {
			Err(errno::ENOTTY)
		}
	}

	/// Synchronizes the file with the device.
	pub fn sync(&self) {
		match &self.location {
			FileLocation::Disk(_l) => {
				// TODO
				// todo!();
			},

			_ => {},
		}
	}

	/// Unlinks the current file.
	pub fn unlink(&mut self) {
		match &self.location {
			FileLocation::Disk(_l) => {
				// TODO
				// todo!();
			},

			_ => {},
		}
	}
}

impl Drop for File {
	fn drop(&mut self) {
		self.sync();
	}
}

/// The access counter allows to count the relative number of accesses count on a file.
pub struct AccessCounter {
	/// The number of accesses to the file relative to the previous file in the pool.
	/// This number is limited by `ACCESSES_UPPER_BOUND`.
	accesses_count: usize,
}

/// Cache storing files in memory. This cache allows to speedup accesses to the disk. It is
/// synchronized with the disk when necessary.
pub struct FCache {
	/// A pointer to the root mount point.
	root_mount: SharedPtr<MountPoint>,

	/// A fixed-size pool storing files, sorted by path.
	files_pool: Vec<File>,
	/// A pool of the same size as the files pool, storing approximate relative accesses count for
	/// each files.
	/// The element at an index is associated to the element in the files pool at the same index.
	accesses_pool: Vec<AccessCounter>,
}

impl FCache {
	/// Creates a new instance with the given major and minor for the root device.
	pub fn new(root_device_type: DeviceType, root_major: u32, root_minor: u32)
		-> Result<Self, Errno> {
		let root_mount = MountPoint::new(root_device_type, root_major, root_minor, 0, Path::root())?;
		let shared_ptr = mountpoint::register_mountpoint(root_mount)?;

		Ok(Self {
			root_mount: shared_ptr,

			files_pool: Vec::<File>::with_capacity(FILES_POOL_SIZE)?,
			accesses_pool: Vec::<AccessCounter>::with_capacity(FILES_POOL_SIZE)?,
		})
	}

	/// Loads the file with the given path `path`. If the file is already loaded, the behaviour is
	/// undefined.
	fn load_file(&mut self, _path: &Path) {
		let len = self.files_pool.len();
		if len >= FILES_POOL_SIZE {
			self.files_pool.pop();
			self.accesses_pool.pop();
		}

		// TODO Push file
	}

	// TODO Use the cache
	/// Adds the file `file` to the VFS. The file will be located into the directory at path
	/// `parent`.
	/// The directory must exist. If an error happens, the function returns an Err with the
	/// appropriate Errno.
	/// If the path is relative, the function starts from the root.
	/// If the file isn't present in the pool, the function shall load it.
	pub fn create_file(&mut self, parent: &Path, file: File) -> Result<SharedPtr<File>, Errno> {
		let mut path = Path::root().concat(parent)?;
		path.reduce()?;

		// Getting the path's deepest mountpoint
		let ptr = mountpoint::get_deepest(&path).ok_or(errno::ENOENT)?;
		let mut guard = ptr.lock(true);
		let deepest_mountpoint = guard.get_mut();

		// Getting the mountpoint's device
		let dev_ptr = deepest_mountpoint.get_device();
		let mut dev_guard = dev_ptr.lock(true);
		let dev = dev_guard.get_mut();

		// Getting the path from the start of the filesystem to the parent directory
		let inner_path = path.range_from(deepest_mountpoint.get_path().get_elements_count()..)?;
		// Getting the parent inode
		let parent_inode = deepest_mountpoint.get_filesystem().get_inode(dev, inner_path)?;

		// Adding the file
		let file = deepest_mountpoint.get_filesystem().add_file(dev, parent_inode, file)?;
		// TODO Set parent
		SharedPtr::new(file)
	}

	// TODO Use the cache
	/// Removes the file at path `path` from the VFS.
	/// If the file is a non-empty directory, the function returns an error.
	pub fn remove_file(&mut self, path: &Path) -> Result<(), Errno> {
		let mut path = Path::root().concat(path)?;
		path.reduce()?;

		// Getting the path's deepest mountpoint
		let ptr = mountpoint::get_deepest(&path).ok_or(errno::ENOENT)?;
		let mut guard = ptr.lock(true);
		let deepest_mountpoint = guard.get_mut();

		// Getting the mountpoint's device
		let dev_ptr = deepest_mountpoint.get_device();
		let mut dev_guard = dev_ptr.lock(true);
		let dev = dev_guard.get_mut();

		let path_len = path.get_elements_count();
		if path_len > 0 {
			let entry_name = &path[path_len - 1];
			let mountpoint_path_len = deepest_mountpoint.get_path().get_elements_count();
			// Getting the path from the start of the fileststem to the parent directory
			let parent_inner_path = path.range(mountpoint_path_len..(path_len - 1))?;

			// Getting the parent inode
			let parent_inode = deepest_mountpoint.get_filesystem().get_inode(dev,
				parent_inner_path)?;
			deepest_mountpoint.get_filesystem().remove_file(dev, parent_inode, entry_name)?;
		}
		Ok(())
	}

	// TODO Use the cache
	/// Returns a reference to the file at path `path`. If the file doesn't exist, the function
	/// returns None.
	/// If the path is relative, the function starts from the root.
	/// If the file isn't present in the pool, the function shall load it.
	pub fn get_file_from_path(&mut self, path: &Path) -> Result<SharedPtr<File>, Errno> {
		let mut path = Path::root().concat(path)?;
		path.reduce()?;

		// Getting the path's deepest mountpoint
		let ptr = mountpoint::get_deepest(&path).ok_or(errno::ENOENT)?;
		let mut guard = ptr.lock(true);
		let deepest_mountpoint = guard.get_mut();

		// Getting the mountpoint's device
		let dev_ptr = deepest_mountpoint.get_device();
		let mut guard = dev_ptr.lock(true);
		let dev = guard.get_mut();

		// Getting the path from the start of the fileststem to the file
		let inner_path = path.range_from(deepest_mountpoint.get_path().get_elements_count()..)?;

		let file = {
			let (entry_name, inode) = if inner_path.is_empty() {
				// Getting the root's inode
				let inode = deepest_mountpoint.get_filesystem().get_inode(dev, Path::root())?;

				(String::new(), inode)
			} else {
				let entry_name = inner_path[inner_path.get_elements_count() - 1].failable_clone()?;
				// Getting the file's inode
				let inode = deepest_mountpoint.get_filesystem().get_inode(dev, inner_path)?;

				(entry_name, inode)
			};

			// Loading the file
			deepest_mountpoint.get_filesystem().load_file(dev, inode, entry_name)
		}?;
		SharedPtr::new(file)
	}
}

/// The instance of the file cache.
static FILES_CACHE: Mutex<Option<FCache>> = Mutex::new(None);

/// Initializes files management.
/// `root_device_type` is the type of the root device file. If not a device, the behaviour is
/// undefined.
/// `root_major` is the major number of the device at the root of the VFS.
/// `root_minor` is the minor number of the device at the root of the VFS.
pub fn init(root_device_type: DeviceType, root_major: u32, root_minor: u32) -> Result<(), Errno> {
	fs::register_defaults()?;

	let cache = FCache::new(root_device_type, root_major, root_minor)?;
	let mut guard = FILES_CACHE.lock(true);
	*guard.get_mut() = Some(cache);

	Ok(())
}

/// Returns a mutable reference to the file cache.
/// If the cache is not initialized, the Option is None.
pub fn get_files_cache() -> &'static Mutex<Option<FCache>> {
	&FILES_CACHE
}
