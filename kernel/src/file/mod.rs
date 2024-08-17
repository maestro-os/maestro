/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Files implementation.
//!
//! The kernel allows *mounting* several filesystems together, which are all unified into one
//! filesystem called the VFS (Virtual FileSystem).
//!
//! The root filesystem is passed to the kernel as an argument on boot.
//! Other filesystems are mounted into subdirectories.

pub mod blocking;
pub mod buffer;
pub mod fd;
pub mod fs;
pub mod mapping;
pub mod mountpoint;
pub mod path;
pub mod perm;
pub mod util;
pub mod vfs;

use crate::{
	device,
	device::{DeviceID, DeviceType},
	file::{
		buffer::{Buffer, BufferOps},
		fs::{Filesystem, NodeOps},
		path::{Path, PathBuf},
		perm::{Gid, Uid},
	},
	syscall::ioctl,
	time::{
		clock,
		clock::CLOCK_MONOTONIC,
		unit::{Timestamp, TimestampScale},
	},
};
use core::{any::Any, ffi::c_void, ops::Deref};
use mountpoint::{MountPoint, MountSource};
use perm::AccessProfile;
use utils::{
	boxed::Box,
	collections::{string::String, vec::Vec},
	errno,
	errno::{AllocResult, EResult},
	lock::Mutex,
	ptr::{arc::Arc, cow::Cow},
	TryClone,
};

/// A filesystem node ID.
///
/// An inode is a number representing a node in a filesystem. The kernel doesn't interpret this
/// value in any ways, but it must fulfill one condition: the value must represent a **unique**
/// node in the filesystem, and that exact node **must** be accessible using this value.
pub type INode = u64;
/// A file mode, which is a pair of values representing respectively:
/// - UNIX permissions (read, write, execute, etc...), represented by the 12 least significant
///   bits.
/// - UNIX type (regular, directory, etc...), represented by the remaining bits.
pub type Mode = u32;

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

/// Directory entry type: Block Device
pub const DT_BLK: u8 = 6;
/// Directory entry type: Char Device
pub const DT_CHR: u8 = 2;
/// Directory entry type: Directory
pub const DT_DIR: u8 = 4;
/// Directory entry type: FIFO
pub const DT_FIFO: u8 = 1;
/// Directory entry type: Symbolic Link
pub const DT_LNK: u8 = 10;
/// Directory entry type: Regular file
pub const DT_REG: u8 = 8;
/// Directory entry type: Socket
pub const DT_SOCK: u8 = 12;
/// Directory entry type: Unknown
pub const DT_UNKNOWN: u8 = 0;

/// Read only.
pub const O_RDONLY: i32 = 0b00000000000000000000000000000000;
/// Write only.
pub const O_WRONLY: i32 = 0b00000000000000000000000000000001;
/// Read and write.
pub const O_RDWR: i32 = 0b00000000000000000000000000000010;
/// At each write operations, the cursor is placed at the end of the file so the
/// data is appended.
pub const O_APPEND: i32 = 0b00000000000000000000010000000000;
/// Generates a SIGIO when input or output becomes possible on the file.
pub const O_ASYNC: i32 = 0b00000000000000000010000000000000;
/// Close-on-exec.
pub const O_CLOEXEC: i32 = 0b00000000000010000000000000000000;
/// If the file doesn't exist, create it.
pub const O_CREAT: i32 = 0b00000000000000000000000001000000;
/// Disables caching data.
pub const O_DIRECT: i32 = 0b00000000000000000100000000000000;
/// If pathname is not a directory, cause the open to fail.
pub const O_DIRECTORY: i32 = 0b00000000000000010000000000000000;
/// Ensure the file is created (when used with O_CREAT). If not, the call fails.
pub const O_EXCL: i32 = 0b00000000000000000000000010000000;
/// Allows openning large files (more than 2^32 bytes).
pub const O_LARGEFILE: i32 = 0b00000000000000001000000000000000;
/// Don't update file access time.
pub const O_NOATIME: i32 = 0b00000000000001000000000000000000;
/// If refering to a tty, it will not become the process's controlling tty.
pub const O_NOCTTY: i32 = 0b00000000000000000000000100000000;
/// Tells `open` not to follow symbolic links.
pub const O_NOFOLLOW: i32 = 0b00000000000000100000000000000000;
/// I/O is non blocking.
pub const O_NONBLOCK: i32 = 0b00000000000000000000100000000000;
/// When using `write`, the data has been transfered to the hardware before
/// returning.
pub const O_SYNC: i32 = 0b00000000000100000001000000000000;
/// If the file already exists, truncate it to length zero.
pub const O_TRUNC: i32 = 0b00000000000000000000001000000000;

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
	///
	/// If the type doesn't exist, the function returns `None`.
	pub const fn from_mode(mode: Mode) -> Option<Self> {
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

	/// Returns the mode corresponding to the type.
	pub const fn to_mode(&self) -> Mode {
		match self {
			Self::Socket => S_IFSOCK,
			Self::Link => S_IFLNK,
			Self::Regular => S_IFREG,
			Self::BlockDevice => S_IFBLK,
			Self::Directory => S_IFDIR,
			Self::CharDevice => S_IFCHR,
			Self::Fifo => S_IFIFO,
		}
	}

	/// Returns the directory entry type.
	pub const fn to_dirent_type(&self) -> u8 {
		match self {
			Self::Socket => DT_SOCK,
			Self::Link => DT_LNK,
			Self::Regular => DT_REG,
			Self::BlockDevice => DT_BLK,
			Self::Directory => DT_DIR,
			Self::CharDevice => DT_CHR,
			Self::Fifo => DT_FIFO,
		}
	}

	/// Returns the device type, if any.
	pub const fn to_device_type(&self) -> Option<DeviceType> {
		match self {
			FileType::BlockDevice => Some(DeviceType::Block),
			FileType::CharDevice => Some(DeviceType::Char),
			_ => None,
		}
	}
}

/// The location of a file.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FileLocation {
	/// The ID of the mountpoint of the file.
	pub mountpoint_id: u32,
	/// The file's inode.
	pub inode: INode,
}

impl FileLocation {
	/// Location to nowhere.
	pub const fn nowhere() -> Self {
		Self {
			mountpoint_id: 0,
			inode: 0,
		}
	}

	/// Returns the mountpoint on which the file is located.
	pub fn get_mountpoint(&self) -> Option<Arc<Mutex<MountPoint>>> {
		mountpoint::from_id(self.mountpoint_id)
	}

	/// Returns the filesystem associated with the location, if any.
	pub fn get_filesystem(&self) -> Option<Arc<dyn Filesystem>> {
		self.get_mountpoint()
			.map(|mp| mp.lock().get_filesystem().clone())
	}
}

/// An entry in a directory, independent of the filesystem type.
#[derive(Debug)]
pub struct DirEntry<'name> {
	/// The entry's inode.
	pub inode: INode,
	/// The entry's type.
	pub entry_type: FileType,
	/// The name of the entry.
	pub name: Cow<'name, [u8]>,
}

impl<'name> TryClone for DirEntry<'name> {
	fn try_clone(&self) -> AllocResult<Self> {
		Ok(Self {
			inode: self.inode,
			entry_type: self.entry_type,
			name: self.name.try_clone()?,
		})
	}
}

/// File status information.
#[derive(Clone, Debug)]
pub struct Stat {
	/// The file's permissions.
	pub mode: Mode,

	/// The number of links to the file.
	pub nlink: u16,

	/// The file owner's user ID.
	pub uid: Uid,
	/// The file owner's group ID.
	pub gid: Gid,

	/// The size of the file in bytes.
	pub size: u64,
	/// The number of blocks occupied by the file.
	pub blocks: u64,

	/// If the file is a device file, this is the major number.
	pub dev_major: u32,
	/// If the file is a device file, this is the minor number.
	pub dev_minor: u32,

	/// Timestamp of the last modification of the metadata.
	pub ctime: Timestamp,
	/// Timestamp of the last modification of the file's content.
	pub mtime: Timestamp,
	/// Timestamp of the last access to the file.
	pub atime: Timestamp,
}

impl Default for Stat {
	fn default() -> Self {
		Self {
			mode: 0o444,

			nlink: 1,

			uid: 0,
			gid: 0,

			size: 0,
			blocks: 0,

			dev_major: 0,
			dev_minor: 0,

			ctime: 0,
			mtime: 0,
			atime: 0,
		}
	}
}

impl Stat {
	/// Returns the file type.
	///
	/// If the file type if invalid, the function returns `None`.
	pub fn get_type(&self) -> Option<FileType> {
		FileType::from_mode(self.mode)
	}

	/// Sets the owner user ID, updating `ctime` with the current timestamp.
	pub fn set_uid(&mut self, uid: Uid) {
		self.uid = uid;
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second).unwrap_or(0);
		self.ctime = timestamp;
	}

	/// Sets the owner group ID, updating `ctime` with the current timestamp.
	pub fn set_gid(&mut self, gid: Gid) {
		self.gid = gid;
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second).unwrap_or(0);
		self.ctime = timestamp;
	}
}

/// An open file description.
#[derive(Debug)]
pub struct File {
	/// The file's absolute path.
	path: PathBuf,
	/// The location the file is stored on.
	location: FileLocation,
	/// Handle to perform operations on the node.
	///
	/// If `None`, the file is virtual.
	ops: Box<dyn NodeOps>,

	/// Open file description flags.
	pub flags: i32,
	/// The current offset in the file.
	pub off: u64,
}

impl File {
	/// Opens a file.
	///
	/// Arguments:
	/// - `path` is the file's absolute path.
	/// - `location` is the file's location.
	/// - `ops` is the handle to perform operations on the file
	/// - `flags` is the open file description's flags.
	pub fn open(
		path: PathBuf,
		location: FileLocation,
		ops: Box<dyn NodeOps>,
		flags: i32,
	) -> EResult<Arc<Mutex<Self>>> {
		let file = Self {
			path,
			location,
			ops,

			flags,
			off: 0,
		};
		Ok(Arc::new(Mutex::new(file))?)
	}

	/// Like [`open`], but without an associated location.
	pub fn open_ops(ops: Box<dyn NodeOps>, flags: i32) -> EResult<Arc<Mutex<Self>>> {
		let file = Self {
			path: PathBuf::empty(),
			location: FileLocation::nowhere(),
			ops,

			flags,
			off: 0,
		};
		Ok(Arc::new(Mutex::new(file))?)
	}

	/// Returns the absolute path to the file.
	pub fn get_path(&self) -> &Path {
		&self.path
	}

	/// Returns the location of the file.
	pub fn get_location(&self) -> &FileLocation {
		&self.location
	}

	/// Returns the mountpoint located at this file, if any.
	pub fn as_mountpoint(&self) -> Option<Arc<Mutex<MountPoint>>> {
		mountpoint::from_location(&self.location)
	}

	/// Tells whether there is a mountpoint on the file.
	pub fn is_mountpoint(&self) -> bool {
		self.as_mountpoint().is_some()
	}

	/// Returns the file's operations handle.
	pub fn ops(&self) -> &dyn NodeOps {
		self.ops.deref()
	}

	pub fn get_stat(&self) -> EResult<Stat> {
		self.ops.get_stat(&self.location)
	}

	/// Returns the type of the file.
	pub fn get_type(&self) -> EResult<FileType> {
		let stat = self.get_stat()?;
		FileType::from_mode(stat.mode).ok_or_else(|| errno!(EUCLEAN))
	}

	/// Returns the file's associated buffer.
	///
	/// If the file does not have a buffer of type `B`, the function returns `None`.
	pub fn get_buffer<B: BufferOps>(&self) -> Option<&B> {
		let buf = (&self.ops as &dyn Any).downcast_ref::<Buffer>()?;
		(buf.0.deref() as &dyn Any).downcast_ref()
	}

	/// Reads the whole content of a file into a buffer.
	///
	/// This function does not change the file's offset.
	pub fn read_all(&mut self) -> EResult<Vec<u8>> {
		let len: usize = self
			.get_stat()?
			.size
			.try_into()
			.map_err(|_| errno!(EOVERFLOW))?;
		let mut buf = Vec::with_capacity(len)?;
		let mut off = 0;
		// Stick to the file's size to have an upper bound
		while off < len {
			let len = self
				.ops
				.read_content(&self.location, off as _, &mut buf[off..])?;
			if len == 0 {
				break;
			}
			off += len;
		}
		Ok(buf)
	}

	pub fn poll(&mut self, mask: u32) -> EResult<u32> {
		self.ops.poll(&self.location, mask)
	}

	/// Returns the directory entry with the given `name`.
	///
	/// If the file is not a directory, the function returns `None`.
	pub fn dir_entry_by_name<'n>(&self, name: &'n [u8]) -> EResult<Option<DirEntry<'n>>> {
		let e = self.ops.entry_by_name(&self.location, name)?;
		Ok(e.map(|(e, ..)| e))
	}

	/// Returns an iterator over the directory's entries.
	///
	/// `start` is the starting offset of the iterator.
	///
	/// If the file is not a directory, the iterator returns nothing.
	pub fn iter_dir_entries(&self, start: u64) -> DirEntryIterator<'_> {
		DirEntryIterator {
			dir: self,
			cursor: start,
		}
	}

	/// Truncates the file to the given `size`.
	///
	/// If `size` is greater than or equals to the current size of the file, the function does
	/// nothing.
	pub fn truncate(&mut self, size: u64) -> EResult<()> {
		self.ops.truncate_content(&self.location, size)
	}

	/// Performs an ioctl operation on the file.
	///
	/// Arguments:
	/// - `mem_space` is the memory space on which pointers are to be dereferenced.
	/// - `request` is the ID of the request to perform.
	/// - `argp` is a pointer to the argument.
	pub fn ioctl(&mut self, request: ioctl::Request, argp: *const c_void) -> EResult<u32> {
		let stat = self.ops.get_stat(&self.location)?;
		let dev_type = stat.get_type().as_ref().and_then(FileType::to_device_type);
		match dev_type {
			Some(dev_type) => {
				let dev = device::get(&DeviceID {
					dev_type,
					major: stat.dev_major,
					minor: stat.dev_minor,
				})
				.ok_or_else(|| errno!(ENODEV))?;
				dev.get_io().ioctl(request, argp)
			}
			None => self.ops.ioctl(&self.location, request, argp),
		}
	}

	/// Closes the file, removing it if removal has been deferred.
	pub fn close(&mut self) -> EResult<()> {
		let stat = self.ops.get_stat(&self.location)?;
		// If no more link remain to the file, remove the node
		if stat.nlink == 0 {
			self.ops.remove_file(&self.location)?;
		}
		Ok(())
	}
}

impl Drop for File {
	fn drop(&mut self) {
		// TODO: kernel log on error?
		let _ = self.close();
	}
}

impl AccessProfile {
	fn check_read_access_impl(uid: Uid, gid: Gid, stat: &Stat) -> bool {
		// If root, bypass checks
		if uid == perm::ROOT_UID || gid == perm::ROOT_GID {
			return true;
		}
		// Check permissions
		if stat.mode & perm::S_IRUSR != 0 && stat.uid == uid {
			return true;
		}
		if stat.mode & perm::S_IRGRP != 0 && stat.gid == gid {
			return true;
		}
		stat.mode & perm::S_IROTH != 0
	}

	/// Tells whether the agent can read a file with the given status.
	///
	/// `effective` tells whether to use effective IDs. If not, real IDs are used.
	pub fn check_read_access(&self, stat: &Stat, effective: bool) -> bool {
		let (uid, gid) = if effective {
			(self.euid, self.egid)
		} else {
			(self.uid, self.gid)
		};
		Self::check_read_access_impl(uid, gid, stat)
	}

	/// Tells whether the agent can read a file with the given status.
	///
	/// This function is the preferred from `check_read_access` for general cases.
	pub fn can_read_file(&self, stat: &Stat) -> bool {
		self.check_read_access(stat, true)
	}

	/// Tells whether the agent can list files of a directory with the given status, **not**
	/// including access to files' contents and metadata.
	#[inline]
	pub fn can_list_directory(&self, stat: &Stat) -> bool {
		self.can_read_file(stat)
	}

	fn check_write_access_impl(uid: Uid, gid: Gid, stat: &Stat) -> bool {
		// If root, bypass checks
		if uid == perm::ROOT_UID || gid == perm::ROOT_GID {
			return true;
		}
		// Check permissions
		if stat.mode & perm::S_IWUSR != 0 && stat.uid == uid {
			return true;
		}
		if stat.mode & perm::S_IWGRP != 0 && stat.gid == gid {
			return true;
		}
		stat.mode & perm::S_IWOTH != 0
	}

	/// Tells whether the agent can write a file with the given status.
	///
	/// `effective` tells whether to use effective IDs. If not, real IDs are used.
	pub fn check_write_access(&self, stat: &Stat, effective: bool) -> bool {
		let (uid, gid) = if effective {
			(self.euid, self.egid)
		} else {
			(self.uid, self.gid)
		};
		Self::check_write_access_impl(uid, gid, stat)
	}

	/// Tells whether the agent can write a file with the given status.
	pub fn can_write_file(&self, stat: &Stat) -> bool {
		self.check_write_access(stat, true)
	}

	/// Tells whether the agent can modify entries in a directory with the given status, including
	/// creating files, deleting files, and renaming files.
	#[inline]
	pub fn can_write_directory(&self, stat: &Stat) -> bool {
		self.can_write_file(stat) && self.can_execute_file(stat)
	}

	fn check_execute_access_impl(uid: Uid, gid: Gid, stat: &Stat) -> bool {
		// If root, bypass checks (unless the file is a regular file)
		if stat.get_type() != Some(FileType::Regular)
			&& (uid == perm::ROOT_UID || gid == perm::ROOT_GID)
		{
			return true;
		}
		// Check permissions
		if stat.mode & perm::S_IXUSR != 0 && stat.uid == uid {
			return true;
		}
		if stat.mode & perm::S_IXGRP != 0 && stat.gid == gid {
			return true;
		}
		stat.mode & perm::S_IXOTH != 0
	}

	/// Tells whether the agent can execute a file with the given status.
	///
	/// `effective` tells whether to use effective IDs. If not, real IDs are used.
	pub fn check_execute_access(&self, stat: &Stat, effective: bool) -> bool {
		let (uid, gid) = if effective {
			(self.euid, self.egid)
		} else {
			(self.uid, self.gid)
		};
		Self::check_execute_access_impl(uid, gid, stat)
	}

	/// Tells whether the agent can execute a file with the given status.
	pub fn can_execute_file(&self, stat: &Stat) -> bool {
		self.check_execute_access(stat, true)
	}

	/// Tells whether the agent can access files of a directory with the given status, *if the name
	/// of the file is known*.
	#[inline]
	pub fn can_search_directory(&self, stat: &Stat) -> bool {
		self.can_execute_file(stat)
	}

	/// Tells whether the agent can set permissions for a file with the given status.
	pub fn can_set_file_permissions(&self, stat: &Stat) -> bool {
		self.euid == perm::ROOT_UID || self.euid == stat.uid
	}
}

/// Iterator over a file's directory entries.
///
/// For each entry, the function also returns the offset to the next.
///
/// If the file is not a directory, the iterator returns nothing.
pub struct DirEntryIterator<'f> {
	/// The directory.
	dir: &'f File,
	/// The current offset in the file.
	cursor: u64,
}

impl<'f> Iterator for DirEntryIterator<'f> {
	type Item = EResult<(DirEntry<'static>, u64)>;

	fn next(&mut self) -> Option<Self::Item> {
		let res = self
			.dir
			.ops
			.next_entry(&self.dir.location, self.cursor)
			.transpose()?;
		match res {
			Ok((entry, off)) => {
				self.cursor = off;
				Some(Ok((entry, self.cursor)))
			}
			Err(e) => Some(Err(e)),
		}
	}
}

/// Initializes files management.
///
/// `root` is the set of major and minor numbers of the root device. If `None`, a tmpfs is used.
pub(crate) fn init(root: Option<(u32, u32)>) -> EResult<()> {
	fs::register_defaults()?;
	// Create the root mountpoint
	let mount_source = match root {
		Some((major, minor)) => MountSource::Device(DeviceID {
			dev_type: DeviceType::Block,
			major,
			minor,
		}),
		None => MountSource::NoDev(String::try_from(b"tmpfs")?),
	};
	mountpoint::create(
		mount_source,
		None,
		0,
		PathBuf::root()?,
		FileLocation::nowhere(),
	)?;
	Ok(())
}

/// Tells whether files management has been initialized.
pub(crate) fn is_init() -> bool {
	!mountpoint::MOUNT_POINTS.lock().is_empty()
}
