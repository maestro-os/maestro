//! The files cache stores files in memory to avoid accessing the disk each times.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::MountPoint;
use crate::file::Uid;
use crate::file::mountpoint;
use crate::file::path::Path;
use crate::limits;
use crate::util::FailableClone;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;
use crate::util::ptr::SharedPtr;

/// The size of the files pool.
const FILES_POOL_SIZE: usize = 1024;

/// Updates the location of the file `file` according to the given mountpoint `mountpoint`.
fn update_location(file: &mut File, mountpoint: &MountPoint) {
	file.get_location_mut().mountpoint_id = Some(mountpoint.get_id());
}

/// Cache storing files in memory. This cache allows to speedup accesses to the disk. It is
/// synchronized with the disk when necessary.
pub struct FCache {
	/// The pool of cached files.
	pool: Vec<SharedPtr<File>>,
	/// The list of free slots in the pool.
	pool_free: Vec<usize>,

	/// Collection mapping file paths to their slot index.
	pool_paths: HashMap<Path, usize>,
	/// Collection mapping a number of accesses to a slot index.
	access_count: Vec<(usize, usize)>,
}

impl FCache {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			pool: Vec::with_capacity(FILES_POOL_SIZE)?,
			pool_free: Vec::new(),

			pool_paths: HashMap::new(),
			access_count: Vec::new(),
		})
	}

	/// Loads the file with the given path `path`. If the file is already loaded, the behaviour is
	/// undefined.
	fn load_file(&mut self, _path: &Path) {
		/*let len = self.pool.len();
		if len >= FILES_POOL_SIZE {
			self.files_pool.pop();
			self.accesses_pool.pop();
		}*/

		// TODO Push file
	}

	/// Synchonizes the cache to the disks, then empties it.
	pub fn flush_all(&mut self) -> Result<(), Errno> {
		// TODO
		todo!();
	}

	// TODO Use the cache
	/// Returns a reference to the file at path `path`. If the file doesn't exist, the function
	/// returns None.
	/// If the path is relative, the function starts from the root.
	/// If the file isn't present in the pool, the function shall load it.
	/// `uid` is the User ID of the user creating the file.
	/// `gid` is the Group ID of the user creating the file.
	/// `follow_links` is true, the function follows symbolic links.
	/// `follows_count` is the number of links that have been followed since the beginning of the
	/// path resolution.
	fn get_file_from_path_(&mut self, path: &Path, uid: Uid, gid: Gid, follow_links: bool,
		follows_count: usize) -> Result<SharedPtr<File>, Errno> {
		let path = Path::root().concat(path)?;

		// Getting the path's deepest mountpoint
		let mountpoint_mutex = mountpoint::get_deepest(&path).ok_or_else(|| errno!(ENOENT))?;
		let mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();
		let mountpath = mountpoint.get_path().failable_clone()?;

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Getting the path from the start of the filesystem to the file
		let inner_path = path.range_from(mountpoint.get_path().get_elements_count()..)?;

		// The filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let fs_guard = fs_mutex.lock();
		let fs = fs_guard.get_mut();

		// The root inode
		let mut inode = fs.get_root_inode(io)?;
		let mut file = fs.load_file(io, inode, String::new())?;
		// If the path is empty, return the root
		if inner_path.is_empty() {
			drop(fs_guard);
			update_location(&mut file, &mountpoint);
			return SharedPtr::new(file);
		}
		// Checking permissions
		if !file.can_read(uid, gid) {
			return Err(errno!(EPERM));
		}

		for i in 0..inner_path.get_elements_count() {
			match inner_path[i].as_bytes() {
				b"." => {},
				b".." => {
					let p = inner_path.range_from((i + 1)..)?;

					drop(fs_guard);
					drop(io_guard);
					drop(mountpoint_guard);
					return self.get_file_from_path_(&p, uid, gid, follow_links, follows_count);
				},

				_ => inode = fs.get_inode(io, Some(inode), &inner_path[i])?,
			}

			// Checking permissions
			file = fs.load_file(io, inode, inner_path[i].failable_clone()?)?;
			if i < inner_path.get_elements_count() - 1 && !file.can_read(uid, gid) {
				return Err(errno!(EPERM));
			}

			// If this is not the last element, or if links are followed
			if i < inner_path.get_elements_count() - 1 || follow_links {
				// If symbolic link, resolve it
				if let FileContent::Link(link_path) = file.get_file_content() {
					if follows_count > limits::SYMLOOP_MAX {
						return Err(errno!(ELOOP));
					}

					let mut prefix = inner_path.range_to(..i)?;
					prefix.set_absolute(false);

					let link_path = Path::from_str(link_path.as_bytes(), false)?;

					let mut suffix = inner_path.range_from((i + 1)..)?;
					suffix.set_absolute(false);

					let parent_path = mountpath.concat(&prefix)?;
					let new_path = parent_path.concat(&link_path)?;
					let new_path = new_path.concat(&suffix)?;

					drop(fs_guard);
					drop(io_guard);
					drop(mountpoint_guard);
					return self.get_file_from_path_(&new_path, uid, gid, follow_links,
						follows_count + 1);
				}
			}
		}

		let mut parent_path = path.failable_clone()?;
		parent_path.pop();
		file.set_parent_path(parent_path);

		drop(fs_guard);
		update_location(&mut file, &mountpoint);
		SharedPtr::new(file)
	}

	// TODO Add a param to choose between the mountpoint and the fs root?
	/// Returns a reference to the file at path `path`. If the file doesn't exist, the function
	/// returns an error.
	/// If the path is relative, the function starts from the root.
	/// If the file isn't present in the pool, the function shall load it.
	/// `uid` is the User ID of the user creating the file.
	/// `gid` is the Group ID of the user creating the file.
	/// `follow_links` is true, the function follows symbolic links.
	pub fn get_file_from_path(&mut self, path: &Path, uid: Uid, gid: Gid, follow_links: bool)
		-> Result<SharedPtr<File>, Errno> {
		self.get_file_from_path_(path, uid, gid, follow_links, 0)
	}

	// TODO Use the cache
	/// Returns a reference to the file `name` located in the directory `parent`. If the file
	/// doesn't exist, the function returns an error.
	/// `parent` is the parent directory.
	/// `name` is the name of the file.
	/// `uid` is the User ID of the user creating the file.
	/// `gid` is the Group ID of the user creating the file.
	/// `follow_links` is true, the function follows symbolic links.
	pub fn get_file_from_parent(&mut self, parent: &mut File, name: String, uid: Uid, gid: Gid,
		follow_links: bool) -> Result<SharedPtr<File>, Errno> {
		// Checking for errors
		if parent.get_file_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !parent.can_read(uid, gid) {
			return Err(errno!(EPERM));
		}

		// Getting the path's deepest mountpoint
		let mountpoint_mutex = parent.get_location().get_mountpoint()
			.ok_or_else(|| errno!(ENOENT))?;
		let mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// The filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let fs_guard = fs_mutex.lock();
		let fs = fs_guard.get_mut();

		let inode = fs.get_inode(io, Some(parent.get_location().inode), &name)?;
		let mut file = fs.load_file(io, inode, name)?;

		if follow_links {
			if let FileContent::Link(link_path) = file.get_file_content() {
				let link_path = Path::from_str(link_path.as_bytes(), false)?;
				let new_path = parent.get_path()?.concat(&link_path)?;

				drop(fs_guard);
				drop(io_guard);
				drop(mountpoint_guard);
				return self.get_file_from_path_(&new_path, uid, gid, follow_links, 1);
			}
		}

		file.set_parent_path(parent.get_path()?);
		update_location(&mut file, &mountpoint);
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
		match self.get_file_from_parent(parent, name.failable_clone()?, uid, gid, false) {
			// If file already exist, error
			Ok(_) => return Err(errno!(EEXIST)),
			// If file doesn't exist, do nothing
			Err(_) => {},
		}

		// Checking for errors
		if parent.get_file_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !parent.can_write(uid, gid) {
			return Err(errno!(EPERM));
		}

		// Getting the mountpoint
		let mountpoint_mutex = parent.get_location().get_mountpoint()
			.ok_or_else(|| errno!(ENOENT))?;
		let mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();
		if mountpoint.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Getting the filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let fs_guard = fs_mutex.lock();
		let fs = fs_guard.get_mut();
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}

		// The parent directory's inode
		let parent_inode = parent.get_location().inode;

		// Adding the file to the filesystem
		let mut file = fs.add_file(io, parent_inode, name, uid, gid, mode, content)?;

		// Adding the file to the parent's entries
		file.set_parent_path(parent.get_path()?);
		parent.add_entry(file.get_name().failable_clone()?, file.to_dir_entry())?;

		drop(fs_guard);
		update_location(&mut file, &mountpoint);
		SharedPtr::new(file)
	}

	/// Creates a new hard link.
	/// `target` is the target file.
	/// `parent` is the parent directory of the new link.
	/// `name` is the name of the link.
	/// `uid` is the id of the owner user.
	/// `gid` is the id of the owner group.
	pub fn create_link(&mut self, target: &mut File, parent: &mut File, name: String)
		-> Result<(), Errno> {
		// Checking the parent file is a directory
		if parent.get_file_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Checking the target and source are both on the same mountpoint
		if target.get_location().mountpoint_id != parent.get_location().mountpoint_id {
			return Err(errno!(EXDEV));
		}

		// Getting the mountpoint
		let mountpoint_mutex = target.get_location().get_mountpoint()
			.ok_or_else(|| errno!(ENOENT))?;
		let mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();
		if mountpoint.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Getting the filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let fs_guard = fs_mutex.lock();
		let fs = fs_guard.get_mut();
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}

		fs.add_link(io, parent.get_location().inode, &name, target.get_location().inode)
		// TODO Update file
	}

	// TODO Use the cache
	/// Removes the file `file` from the VFS.
	/// If the file doesn't exist, the function returns an error.
	/// If the file is a non-empty directory, the function returns an error.
	/// `uid` is the User ID of the user removing the file.
	/// `gid` is the Group ID of the user removing the file.
	pub fn remove_file(&mut self, file: &File, uid: Uid, gid: Gid) -> Result<(), Errno> {
		if file.is_busy() {
			return Err(errno!(EBUSY));
		}

		// The parent directory.
		let parent_mutex = self.get_file_from_path(file.get_parent_path(), uid, gid, true)?;
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get();
		let parent_inode = parent.get_location().inode;

		// Checking permissions
		if !file.can_write(uid, gid) || !parent.can_write(uid, gid) {
			return Err(errno!(EPERM));
		}

		// Getting the mountpoint
		let mountpoint_mutex = file.get_location()
			.get_mountpoint()
			.ok_or_else(|| errno!(ENOENT))?;
		let mountpoint_guard = mountpoint_mutex.lock();
		let mountpoint = mountpoint_guard.get_mut();
		if mountpoint.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let io_guard = io_mutex.lock();
		let io = io_guard.get_mut();

		// Getting the filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let fs_guard = fs_mutex.lock();
		let fs = fs_guard.get_mut();
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Removing the file
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
