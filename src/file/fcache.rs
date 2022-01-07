//! The files cache stores files in memory to avoid accessing the disk each times.

use crate::device::Device;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
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

// TODO Check files/directories access permissions when getting, creating, removing, etc...
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
		let mut guard = ptr.lock();
		let mountpoint = guard.get_mut();

		// Getting the path from the start of the filesystem to the parent directory
		let inner_path = path.range_from(mountpoint.get_path().get_elements_count()..)?;

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io();
		let mut io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		let fs = mountpoint.get_filesystem();
		if fs.is_readonly() {
			return Err(errno::EROFS);
		}

		let parent_inode = fs.get_inode(io, inner_path)?;
		let file = fs.add_file(io, parent_inode, file)?;

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
		let mut guard = ptr.lock();
		let mountpoint = guard.get_mut();

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io();
		let mut io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		let path_len = path.get_elements_count();
		if path_len > 0 {
			let entry_name = &path[path_len - 1];
			let mountpoint_path_len = mountpoint.get_path().get_elements_count();
			// Getting the path from the start of the fileststem to the parent directory
			let parent_inner_path = path.range(mountpoint_path_len..(path_len - 1))?;

			let fs = mountpoint.get_filesystem();
			if fs.is_readonly() {
				return Err(errno::EROFS);
			}

			// Getting the parent inode
			let parent_inode = fs.get_inode(io, parent_inner_path)?;
			fs.remove_file(io, parent_inode, entry_name)?;
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
		let mut guard = ptr.lock();
		let mountpoint = guard.get_mut();

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io();
		let mut io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Getting the path from the start of the fileststem to the file
		let inner_path = path.range_from(mountpoint.get_path().get_elements_count()..)?;

		let fs = mountpoint.get_filesystem();

		let file = {
			let (entry_name, inode) = if inner_path.is_empty() {
				// Getting the root's inode
				let inode = fs.get_inode(io, Path::root())?;

				(String::new(), inode)
			} else {
				let entry_name = inner_path[inner_path.get_elements_count() - 1].failable_clone()?;
				// Getting the file's inode
				let inode = fs.get_inode(io, inner_path)?;

				(entry_name, inode)
			};

			// Loading the file
			fs.load_file(io, inode, entry_name)
		}?;
		SharedPtr::new(file)
	}
}

/// The instance of the file cache.
static FILES_CACHE: Mutex<Option<FCache>> = Mutex::new(None);

/// Returns a mutable reference to the file cache.
/// If the cache is not initialized, the Option is None.
pub fn get() -> &'static Mutex<Option<FCache>> {
	&FILES_CACHE
}
