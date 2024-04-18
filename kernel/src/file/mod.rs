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
pub mod open_file;
pub mod path;
pub mod perm;
pub mod util;
pub mod vfs;

use crate::{
	device,
	device::{DeviceID, DeviceType},
	file::{
		buffer::{pipe::PipeBuffer, socket::Socket},
		fs::{Filesystem, NodeOps, StatSet},
		path::PathBuf,
		perm::{Gid, Uid},
	},
	process::mem_space::MemSpace,
	syscall::ioctl,
	time::{
		clock,
		clock::CLOCK_MONOTONIC,
		unit::{Timestamp, TimestampScale},
	},
};
use core::{cmp::max, ffi::c_void};
use mountpoint::{MountPoint, MountSource};
use perm::AccessProfile;
use utils::{
	boxed::Box,
	collections::string::String,
	errno,
	errno::{AllocResult, EResult},
	io::IO,
	lock::{IntMutex, Mutex},
	ptr::{arc::Arc, cow::Cow},
	vec, TryClone,
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

	/// Returns the mode corresponding to the type.
	pub fn to_mode(&self) -> Mode {
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
	pub fn to_dirent_type(&self) -> u8 {
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
}

/// The location of a file on a disk.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FileLocation {
	/// The file is located on a filesystem.
	Filesystem {
		/// The ID of the mountpoint of the file.
		mountpoint_id: u32,
		/// The file's inode.
		inode: INode,
	},
	/// The file is not located on a filesystem.
	///
	/// This variant contains an ID.
	Virtual(u32),
}

impl FileLocation {
	/// Dummy location, to be used by the root mountpoint.
	pub const fn dummy() -> Self {
		Self::Filesystem {
			mountpoint_id: 0,
			inode: 0,
		}
	}

	/// Returns the ID of the mountpoint.
	pub fn get_mountpoint_id(&self) -> Option<u32> {
		match self {
			Self::Filesystem {
				mountpoint_id, ..
			} => Some(*mountpoint_id),
			_ => None,
		}
	}

	/// Returns the mountpoint on which the file is located.
	pub fn get_mountpoint(&self) -> Option<Arc<Mutex<MountPoint>>> {
		mountpoint::from_id(self.get_mountpoint_id()?)
	}

	/// Returns the inode.
	pub fn get_inode(&self) -> INode {
		match self {
			Self::Filesystem {
				inode, ..
			} => *inode,
			Self::Virtual(id) => *id as _,
		}
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
	/// The file's type.
	///
	/// This field **must not** be modified after the structure is initialized.
	pub file_type: FileType,
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
			file_type: FileType::Regular,
			mode: 0o444,

			nlink: 0,

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
	/// Sets the permissions of the file, updating `ctime` with the current timestamp.
	pub fn set_permissions(&mut self, mode: Mode) {
		self.mode = mode & 0o7777;
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second).unwrap_or(0);
		self.ctime = timestamp;
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

/// Information to remove a file when all its handles are closed.
#[derive(Debug)]
pub struct DeferredRemove {
	/// The the parent directory.
	pub parent: Arc<Mutex<File>>,
	/// The name of the entry to remove.
	pub name: String,
}

/// I/O source given by [`File::io_op`].
enum IoSource<'a> {
	/// The operation is performed on a filesystem.
	Filesystem {
		/// The filesystem.
		fs: &'a dyn Filesystem,
		/// The inode on the filesystem.
		inode: INode,
		/// Handle to perform operations on the filesystem's node.
		ops: &'a dyn NodeOps,
	},
	/// The operation is performed on a simple I/O interface.
	IO(&'a mut dyn IO),
}

/// A file on a filesystem.
///
/// This structure does not store the file's name as it may be different depending on the hard link
/// used to access it.
#[derive(Debug)]
pub struct File {
	/// The location the file is stored on.
	pub location: FileLocation,
	/// The file's status. This is cache of the data on the filesystem.
	pub stat: Stat,
	/// Handle to perform operations on the node.
	ops: Box<dyn NodeOps>,
	/// If not `None`, the file will be removed when the last handle to it is closed.
	///
	/// This field contains all the information necessary to remove it.
	deferred_remove: Option<DeferredRemove>,
}

impl File {
	/// Creates a new instance.
	///
	/// Arguments:
	/// - `location` is the file's location.
	/// - `stat` is the file's status
	/// - `ops` is the handle to perform operations on the file
	fn new(location: FileLocation, stat: Stat, ops: Box<dyn NodeOps>) -> Self {
		Self {
			location,
			stat,
			ops,
			deferred_remove: None,
		}
	}

	/// Returns the file's location.
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

	/// Returns the directory entry with the given `name`.
	///
	/// If the file is not a directory, the function returns `None`.
	pub fn dir_entry_by_name<'n>(&self, name: &'n [u8]) -> EResult<Option<DirEntry<'n>>> {
		self.io_op(|io| {
			let e = match io {
				IoSource::Filesystem {
					fs,
					inode,
					ops,
				} => ops.entry_by_name(inode, fs, name),
				IoSource::IO(_) => Ok(None),
			}?;
			Ok(e.map(|(e, ..)| e))
		})
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

	/// Reads the symbolic link.
	///
	/// If the file is not a symbolic link, the function returns [`errno::EINVAL`].
	pub fn read_link(&mut self) -> EResult<PathBuf> {
		if self.stat.file_type != FileType::Link {
			return Err(errno!(EINVAL));
		}
		let mut link_path = vec![0; self.stat.size as usize]?;
		self.read(0, &mut link_path)?;
		Ok(PathBuf::new_unchecked(String::from(link_path)))
	}

	/// Truncates the file to the given `size`.
	///
	/// If `size` is greater than or equals to the current size of the file, the function does
	/// nothing.
	pub fn truncate(&mut self, size: u64) -> EResult<()> {
		self.io_op(|io| match io {
			IoSource::Filesystem {
				fs,
				inode,
				ops,
			} => ops.truncate_content(inode, fs, size),
			IoSource::IO(_) => Err(errno!(EINVAL)),
		})
	}

	/// Performs an ioctl operation on the file.
	///
	/// Arguments:
	/// - `mem_space` is the memory space on which pointers are to be dereferenced.
	/// - `request` is the ID of the request to perform.
	/// - `argp` is a pointer to the argument.
	pub fn ioctl(
		&mut self,
		mem_space: Arc<IntMutex<MemSpace>>,
		request: ioctl::Request,
		argp: *const c_void,
	) -> EResult<u32> {
		match self.stat.file_type {
			FileType::Fifo => {
				let buff_mutex = buffer::get_or_default::<PipeBuffer>(&self.location)?;
				let mut buff = buff_mutex.lock();
				buff.ioctl(mem_space, request, argp)
			}
			FileType::Socket => {
				let buff_mutex = buffer::get_or_default::<Socket>(&self.location)?;
				let mut buff = buff_mutex.lock();
				buff.ioctl(mem_space, request, argp)
			}
			FileType::BlockDevice => {
				let dev_mutex = device::get(&DeviceID {
					dev_type: DeviceType::Block,
					major: self.stat.dev_major,
					minor: self.stat.dev_minor,
				})
				.ok_or_else(|| errno!(ENODEV))?;
				let mut dev = dev_mutex.lock();
				dev.get_handle().ioctl(mem_space, request, argp)
			}
			FileType::CharDevice => {
				let dev_mutex = device::get(&DeviceID {
					dev_type: DeviceType::Char,
					major: self.stat.dev_major,
					minor: self.stat.dev_minor,
				})
				.ok_or_else(|| errno!(ENODEV))?;
				let mut dev = dev_mutex.lock();
				dev.get_handle().ioctl(mem_space, request, argp)
			}
			_ => Err(errno!(ENOTTY)),
		}
	}

	/// Synchronizes the file with the device.
	///
	/// If no device is associated with the file, the function does nothing.
	pub fn sync(&self) -> EResult<()> {
		let inode = self.location.get_inode();
		let Some(mountpoint_mutex) = self.location.get_mountpoint() else {
			return Ok(());
		};
		let mountpoint = mountpoint_mutex.lock();
		let fs = mountpoint.get_filesystem();
		// TODO only set fields that were modified
		self.ops.set_stat(
			inode,
			&*fs,
			StatSet {
				mode: Some(self.stat.mode),
				nlink: None,
				uid: Some(self.stat.uid),
				gid: Some(self.stat.gid),
				ctime: Some(self.stat.ctime),
				mtime: Some(self.stat.mtime),
				atime: Some(self.stat.atime),
			},
		)
	}

	/// Wrapper for I/O operations on files.
	///
	/// For the current file, the function takes a closure which provides the following arguments:
	/// - The I/O interface to write the file, if any.
	/// - The filesystem of the file, if any.
	fn io_op<R, F>(&self, f: F) -> EResult<R>
	where
		F: FnOnce(IoSource<'_>) -> EResult<R>,
	{
		match self.stat.file_type {
			FileType::Regular => match self.location {
				FileLocation::Filesystem {
					inode, ..
				} => {
					let fs = {
						let mountpoint_mutex =
							self.location.get_mountpoint().ok_or_else(|| errno!(EIO))?;
						let mountpoint = mountpoint_mutex.lock();
						mountpoint.get_filesystem()
					};
					f(IoSource::Filesystem {
						fs: &*fs,
						inode,
						ops: &*self.ops,
					})
				}
				FileLocation::Virtual {
					..
				} => {
					let Some(io_mutex) = buffer::get(&self.location) else {
						return Err(errno!(ENOENT));
					};
					let mut io = io_mutex.lock();
					f(IoSource::IO(&mut *io))
				}
			},
			FileType::Directory => Err(errno!(EISDIR)),
			FileType::Link => Err(errno!(EINVAL)),
			FileType::Fifo => {
				let io_mutex = buffer::get_or_default::<PipeBuffer>(&self.location)?;
				let mut io = io_mutex.lock();
				f(IoSource::IO(&mut *io))
			}
			FileType::Socket => {
				let io_mutex = buffer::get_or_default::<Socket>(&self.location)?;
				let mut io = io_mutex.lock();
				f(IoSource::IO(&mut *io))
			}
			FileType::BlockDevice => {
				let io_mutex = device::get(&DeviceID {
					dev_type: DeviceType::Block,
					major: self.stat.dev_major,
					minor: self.stat.dev_minor,
				})
				.ok_or_else(|| errno!(ENODEV))?;
				let mut io = io_mutex.lock();
				f(IoSource::IO(&mut *io))
			}
			FileType::CharDevice => {
				let io_mutex = device::get(&DeviceID {
					dev_type: DeviceType::Char,
					major: self.stat.dev_major,
					minor: self.stat.dev_minor,
				})
				.ok_or_else(|| errno!(ENODEV))?;
				let mut io = io_mutex.lock();
				f(IoSource::IO(&mut *io))
			}
		}
	}

	/// Defers removal of the file, meaning the file will be removed when closed.
	pub fn defer_remove(&mut self, info: DeferredRemove) {
		self.deferred_remove = Some(info);
	}

	/// Closes the file, removing it if removal has been deferred.
	pub fn close(&mut self) -> EResult<()> {
		if let Some(deferred_remove) = self.deferred_remove.take() {
			// No need to check permissions since they already have been checked before deferring
			let parent = deferred_remove.parent.lock();
			vfs::remove_file_unchecked(&parent, &deferred_remove.name)?;
		}
		Ok(())
	}
}

impl IO for File {
	fn get_size(&self) -> u64 {
		self.stat.size
	}

	fn read(&mut self, off: u64, buff: &mut [u8]) -> EResult<(u64, bool)> {
		self.io_op(|io| match io {
			IoSource::Filesystem {
				fs,
				inode,
				ops,
			} => ops.read_content(inode, fs, off, buff),
			IoSource::IO(io) => io.read(off, buff),
		})
		// TODO update `atime` if the mountpoint allows it
	}

	fn write(&mut self, off: u64, buff: &[u8]) -> EResult<u64> {
		let len = self.io_op(|io| match io {
			IoSource::Filesystem {
				fs,
				inode,
				ops,
			} => ops.write_content(inode, fs, off, buff),
			IoSource::IO(io) => io.write(off, buff),
		})?;
		// Update file's size
		self.stat.size = max(off + len, self.stat.size);
		// TODO update `blocks`
		// TODO update `mtime`
		Ok(len)
	}

	fn poll(&mut self, mask: u32) -> EResult<u32> {
		self.io_op(|io| {
			match io {
				IoSource::Filesystem {
					fs: _,
					inode: _,
					ops: _,
				} => {
					// TODO
					todo!()
				}
				IoSource::IO(io) => io.poll(mask),
			}
		})
	}
}

impl Drop for File {
	fn drop(&mut self) {
		// TODO: kernel log on error?
		let _ = self.close();
	}
}

impl AccessProfile {
	fn check_read_access_impl(uid: Uid, gid: Gid, file: &File) -> bool {
		// If root, bypass checks
		if uid == perm::ROOT_UID || gid == perm::ROOT_GID {
			return true;
		}
		// Check permissions
		if file.stat.mode & perm::S_IRUSR != 0 && file.stat.uid == uid {
			return true;
		}
		if file.stat.mode & perm::S_IRGRP != 0 && file.stat.gid == gid {
			return true;
		}
		file.stat.mode & perm::S_IROTH != 0
	}

	/// Tells whether the agent can read the file.
	///
	/// `effective` tells whether to use effective IDs. If not, real IDs are used.
	pub fn check_read_access(&self, file: &File, effective: bool) -> bool {
		let (uid, gid) = if effective {
			(self.get_euid(), self.get_egid())
		} else {
			(self.get_uid(), self.get_gid())
		};
		Self::check_read_access_impl(uid, gid, file)
	}

	/// Tells whether the agent can read the file.
	///
	/// This function is the preferred from `check_read_access` for general cases.
	pub fn can_read_file(&self, file: &File) -> bool {
		self.check_read_access(file, true)
	}

	/// Tells whether the agent can list files of the directories, **not** including access to
	/// files' contents and metadata.
	#[inline]
	pub fn can_list_directory(&self, file: &File) -> bool {
		self.can_read_file(file)
	}

	fn check_write_access_impl(uid: Uid, gid: Gid, file: &File) -> bool {
		// If root, bypass checks
		if uid == perm::ROOT_UID || gid == perm::ROOT_GID {
			return true;
		}
		// Check permissions
		if file.stat.mode & perm::S_IWUSR != 0 && file.stat.uid == uid {
			return true;
		}
		if file.stat.mode & perm::S_IWGRP != 0 && file.stat.gid == gid {
			return true;
		}
		file.stat.mode & perm::S_IWOTH != 0
	}

	/// Tells whether the agent can write the file.
	///
	/// `effective` tells whether to use effective IDs. If not, real IDs are used.
	pub fn check_write_access(&self, file: &File, effective: bool) -> bool {
		let (uid, gid) = if effective {
			(self.get_euid(), self.get_egid())
		} else {
			(self.get_uid(), self.get_gid())
		};
		Self::check_write_access_impl(uid, gid, file)
	}

	/// Tells whether the agent can write the file.
	pub fn can_write_file(&self, file: &File) -> bool {
		self.check_write_access(file, true)
	}

	/// Tells whether the agent can modify entries in the directory, including creating files,
	/// deleting files, and renaming files.
	#[inline]
	pub fn can_write_directory(&self, file: &File) -> bool {
		self.can_write_file(file) && self.can_execute_file(file)
	}

	fn check_execute_access_impl(uid: Uid, gid: Gid, file: &File) -> bool {
		// If root, bypass checks (unless the file is a regular file)
		if file.stat.file_type != FileType::Regular
			&& (uid == perm::ROOT_UID || gid == perm::ROOT_GID)
		{
			return true;
		}
		// Check permissions
		if file.stat.mode & perm::S_IXUSR != 0 && file.stat.uid == uid {
			return true;
		}
		if file.stat.mode & perm::S_IXGRP != 0 && file.stat.gid == gid {
			return true;
		}
		file.stat.mode & perm::S_IXOTH != 0
	}

	/// Tells whether the agent can execute the file.
	///
	/// `effective` tells whether to use effective IDs. If not, real IDs are used.
	pub fn check_execute_access(&self, file: &File, effective: bool) -> bool {
		let (uid, gid) = if effective {
			(self.get_euid(), self.get_egid())
		} else {
			(self.get_uid(), self.get_gid())
		};
		Self::check_execute_access_impl(uid, gid, file)
	}

	/// Tells whether the agent can execute the file.
	pub fn can_execute_file(&self, file: &File) -> bool {
		self.check_execute_access(file, true)
	}

	/// Tells whether the agent can access files of the directory *if the name of the file is
	/// known*.
	#[inline]
	pub fn can_search_directory(&self, file: &File) -> bool {
		self.can_execute_file(file)
	}

	/// Tells whether the agent can set permissions for the given file.
	pub fn can_set_file_permissions(&self, file: &File) -> bool {
		let euid = self.get_euid();
		euid == perm::ROOT_UID || euid == file.stat.uid
	}
}

/// Iterator over a file's directory entries.
///
/// If the file is not a directory, the iterator returns nothing.
pub struct DirEntryIterator<'f> {
	/// The directory.
	dir: &'f File,
	/// The current offset in the file.
	cursor: u64,
}

impl<'f> Iterator for DirEntryIterator<'f> {
	type Item = EResult<DirEntry<'static>>;

	fn next(&mut self) -> Option<Self::Item> {
		let res = self
			.dir
			.io_op(|io| match io {
				IoSource::Filesystem {
					fs,
					inode,
					ops,
				} => ops.next_entry(inode, fs, self.cursor),
				IoSource::IO(_) => Err(errno!(ENOTDIR)),
			})
			.transpose()?;
		match res {
			Ok((entry, off)) => {
				self.cursor = off;
				Some(Ok(entry))
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
		PathBuf::root(),
		FileLocation::dummy(),
	)?;
	Ok(())
}

/// Tells whether files management has been initialized.
pub(crate) fn is_init() -> bool {
	!mountpoint::MOUNT_POINTS.lock().is_empty()
}
