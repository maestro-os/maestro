//! The files cache stores files in memory to avoid accessing the disk each times.

use crate::device::Device;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::mountpoint::MountPoint;
use crate::file::mountpoint::MountSource;
use crate::file::mountpoint;
use crate::file::path::Path;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;

/// The size of the files pool.
const FILES_POOL_SIZE: usize = 1024;
/// The upper bount for the file accesses counter.
const ACCESSES_UPPER_BOUND: usize = 128;

/// The access counter allows to count the relative number of accesses count on a file.
struct AccessCounter {
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
	/// Creates a new instance.
	/// `root_device` is the device for the root of the VFS.
	pub fn new(root_device: SharedPtr<Device>) -> Result<Self, Errno> {
		let mount_source = MountSource::Device(root_device);
		let root_mount = MountPoint::new(mount_source, None, 0, Path::root())?;
		let shared_ptr = mountpoint::register(root_mount)?;

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
	// TODO Add a param to choose between the mountpoint and the fs root?
	/// Returns a reference to the file at path `path`. If the file doesn't exist, the function
	/// returns None.
	/// If the path is relative, the function starts from the root.
	/// If the file isn't present in the pool, the function shall load it.
	/// `uid` is the User ID of the user creating the file.
	/// `gid` is the Group ID of the user creating the file.
	pub fn get_file_from_path(&mut self, path: &Path, uid: Uid, gid: Gid)
		-> Result<SharedPtr<File>, Errno> {
		let mut path = Path::root().concat(path)?;
		path.reduce()?;

		// Getting the path's deepest mountpoint
		let mountpoint_mutex = mountpoint::get_deepest(&path).ok_or(errno!(ENOENT))?;
		let mut mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io();
		let mut io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Getting the path from the start of the fileststem to the file
		let inner_path = path.range_from(mountpoint.get_path().get_elements_count()..)?;

		// The filesystem
		let fs = mountpoint.get_filesystem();

		// The current inode
		let mut inode = fs.get_inode(io, None, None)?;

		let file = {
			// If the path is empty, return the root
			if inner_path.is_empty() {
				fs.load_file(io, inode, String::new())?
			} else {
				for i in 0..inner_path.get_elements_count() {
					let name = &inner_path[i];
					let file = fs.load_file(io, inode, name.failable_clone()?)?;

					// Checking permissions
					if i < inner_path.get_elements_count() - 1 && !file.can_read(uid, gid) {
						return Err(errno!(EPERM));
					}

					inode = fs.get_inode(io, Some(inode), Some(name))?;
				}

				let name = &inner_path[inner_path.get_elements_count() - 1];
				fs.load_file(io, inode, name.failable_clone()?)?
			}
		};

		SharedPtr::new(file)
	}

	// TODO Use the cache
	/// Creates a file, adds it to the VFS, then returns it. The file will be located into the
	/// directory `parent`.
	/// If `parent` is not a directory, the function returns an error.
	/// `name` is the name of the file.
	/// `uid` is the id of the owner user.
	/// `gid` is the id of the owner group.
	/// `mode` is the permission of the file.
	/// `content` is the content of the file. This value also determines the file type.
	pub fn create_file(&mut self, parent: &mut File, name: String, uid: Uid, gid: Gid, mode: Mode,
		content: FileContent) -> Result<SharedPtr<File>, Errno> {
		// Checking for errors
		if parent.get_file_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !parent.can_write(uid, gid) {
			return Err(errno!(EPERM));
		}

		// Getting the mountpoint
		let mountpoint_mutex = parent.get_location().get_mountpoint().ok_or(errno!(ENOENT))?;
		let mut mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io();
		let mut io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		let fs = mountpoint.get_filesystem();
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}

		// The parent directory's inode
		let parent_inode = parent.get_location().get_inode();
		// Adding the file to the filesystem
		let mut file = fs.add_file(io, parent_inode, name, uid, gid, mode, content)?;

		// Adding the file to the parent's subfiles
		file.set_parent_path(parent.get_path()?);
		parent.add_subfile(file.get_name().failable_clone()?)?;

		SharedPtr::new(file)
	}

	// TODO Use the cache
	/// Removes the file `file` from the VFS.
	/// If the file doesn't exist, the function returns an error.
	/// If the file is a non-empty directory, the function returns an error.
	/// `uid` is the User ID of the user removing the file.
	/// `gid` is the Group ID of the user removing the file.
	pub fn remove_file(&mut self, file: &File, uid: Uid, gid: Gid) -> Result<(), Errno> {
		// The parent directory.
		let parent_mutex = self.get_file_from_path(file.get_parent_path(), uid, gid)?;
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get();
		let parent_inode = parent.get_location().get_inode();

		// Checking permissions
		if !file.can_write(uid, gid) || !parent.can_write(uid, gid) {
			return Err(errno!(EPERM));
		}

		// Getting the mountpoint
		let mountpoint_mutex = file.get_location().get_mountpoint().ok_or(errno!(ENOENT))?;
		let mut mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io();
		let mut io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Removing the file
		let fs = mountpoint.get_filesystem();
		fs.remove_file(io, parent_inode, file.get_name())?;

		Ok(())
	}
}

/// The instance of the file cache.
static FILES_CACHE: Mutex<Option<FCache>> = Mutex::new(None);

/// Returns a mutable reference to the file cache.
/// If the cache is not initialized, the Option is None.
pub fn get() -> &'static Mutex<Option<FCache>> {
	&FILES_CACHE
}
