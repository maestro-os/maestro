//! The VFS (Virtual FileSystem) is a entity which aggregates every mounted
//! filesystems into one.
//!
//! To manipulate files, the VFS should be used instead of
//! calling the filesystems' functions directly.

use crate::errno;
use crate::errno::Errno;
use crate::file;
use crate::file::buffer;
use crate::file::mapping::FileMappingManager;
use crate::file::mountpoint;
use crate::file::path::Path;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::MountPoint;
use crate::file::Uid;
use crate::limits;
use crate::util::container::string::String;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryClone;
use core::ptr::NonNull;

/// Updates the location of the file `file` according to the given mountpoint
/// `mountpoint`.
///
/// If the file in not located on a filesystem, the function does nothing.
fn update_location(file: &mut File, mountpoint: &MountPoint) {
	if let FileLocation::Filesystem {
		mountpoint_id, ..
	} = file.get_location_mut()
	{
		*mountpoint_id = Some(mountpoint.get_id());
	}
}

/// The Virtual FileSystem.
///
/// This structure acts as an aggregator of every mounted filesystems, but also
/// as a cache to speedup file accesses.
pub struct VFS {
	// TODO Add files caching
	/// Structure managing file mappings.
	file_mappings_manager: FileMappingManager,
}

impl VFS {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			file_mappings_manager: FileMappingManager::new(),
		}
	}

	/// Returns the file corresponding to the given location `location`.
	///
	/// This function doesn't set the name of the file since it cannot be known solely on its
	/// location.
	///
	/// If the file doesn't exist, the function returns an error.
	pub fn get_file_by_location(
		&mut self,
		location: &FileLocation,
	) -> Result<Arc<Mutex<File>>, Errno> {
		match location {
			FileLocation::Filesystem {
				inode, ..
			} => {
				// Getting the mountpoint
				let mountpoint_mutex = location.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
				let mountpoint = mountpoint_mutex.lock();

				// Getting the IO interface
				let io_mutex = mountpoint.get_source().get_io()?;
				let mut io = io_mutex.lock();

				// The filesystem
				let fs_mutex = mountpoint.get_filesystem();
				let mut fs = fs_mutex.lock();

				let mut file = fs.load_file(&mut *io, *inode, String::new())?;

				update_location(&mut file, &mountpoint);
				Arc::new(Mutex::new(file))
			}

			FileLocation::Virtual {
				id,
			} => {
				let name = crate::format!("virtual:{}", id)?;
				let content = FileContent::Fifo; // TODO

				Arc::new(Mutex::new(File::new(
					name,
					0, // TODO
					0, // TODO
					0o666,
					location.clone(),
					content,
				)?))
			}
		}
	}

	// TODO Use the cache
	/// Internal version of `get_file_from_path_`.
	///
	/// `follows_count` is the number of links that have been followed since the
	/// beginning of the path resolution.
	fn get_file_from_path_(
		&mut self,
		path: &Path,
		uid: Uid,
		gid: Gid,
		follow_links: bool,
		follows_count: usize,
	) -> Result<Arc<Mutex<File>>, Errno> {
		let path = Path::root().concat(path)?;

		// Getting the path's deepest mountpoint
		let mountpoint_mutex = mountpoint::get_deepest(&path).ok_or_else(|| errno!(ENOENT))?;
		let mountpoint = mountpoint_mutex.lock();
		let mountpath = mountpoint.get_path().try_clone()?;

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let mut io = io_mutex.lock();

		// Getting the path from the start of the filesystem to the file
		let inner_path = path.range_from(mountpoint.get_path().get_elements_count()..)?;

		// The filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let mut fs = fs_mutex.lock();

		// The root inode
		let mut inode = fs.get_root_inode(&mut *io)?;
		let mut file = fs.load_file(&mut *io, inode, String::new())?;
		// If the path is empty, return the root
		if inner_path.is_empty() {
			drop(fs);

			update_location(&mut file, &mountpoint);
			return Arc::new(Mutex::new(file));
		}
		// Checking permissions
		if !file.can_execute(uid, gid) {
			return Err(errno!(EACCES));
		}

		for i in 0..inner_path.get_elements_count() {
			inode = fs.get_inode(&mut *io, Some(inode), &inner_path[i])?;

			// Checking permissions
			file = fs.load_file(&mut *io, inode, inner_path[i].try_clone()?)?;
			if i < inner_path.get_elements_count() - 1 && !file.can_execute(uid, gid) {
				return Err(errno!(EACCES));
			}

			// If this is not the last element, or if links are followed
			if i < inner_path.get_elements_count() - 1 || follow_links {
				// If symbolic link, resolve it
				if let FileContent::Link(link_path) = file.get_content() {
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

					drop(fs);
					drop(io);
					drop(mountpoint);
					return self.get_file_from_path_(
						&new_path,
						uid,
						gid,
						follow_links,
						follows_count + 1,
					);
				}
			}
		}

		let mut parent_path = path.try_clone()?;
		parent_path.pop();
		file.set_parent_path(parent_path);

		drop(fs);

		update_location(&mut file, &mountpoint);
		Arc::new(Mutex::new(file))
	}

	// TODO Add a param to choose between the mountpoint and the fs root?
	/// Returns a reference to the file at path `path`.
	///
	/// If the file doesn't exist, the function returns an error.
	///
	/// If the path is relative, the function starts from the root.
	///
	/// Arguments:
	/// - `uid` is the User ID of the user creating the file.
	/// - `gid` is the Group ID of the user creating the file.
	/// - `follow_links` is `true`, the function follows symbolic links.
	pub fn get_file_from_path(
		&mut self,
		path: &Path,
		uid: Uid,
		gid: Gid,
		follow_links: bool,
	) -> Result<Arc<Mutex<File>>, Errno> {
		self.get_file_from_path_(path, uid, gid, follow_links, 0)
	}

	// TODO Use the cache
	/// Returns a reference to the file `name` located in the directory `parent`.
	///
	/// If the file doesn't exist, the function returns an error.
	///
	/// Arguments:
	/// - `parent` is the parent directory.
	/// - `name` is the name of the file.
	/// - `uid` is the User ID of the user creating the file.
	/// - `gid` is the Group ID of the user creating the file.
	/// - `follow_links` is `true`, the function follows symbolic links.
	pub fn get_file_from_parent(
		&mut self,
		parent: &mut File,
		name: String,
		uid: Uid,
		gid: Gid,
		follow_links: bool,
	) -> Result<Arc<Mutex<File>>, Errno> {
		// Checking for errors
		if parent.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !parent.can_execute(uid, gid) {
			return Err(errno!(EACCES));
		}

		// Getting the path's deepest mountpoint
		let mountpoint_mutex = parent
			.get_location()
			.get_mountpoint()
			.ok_or_else(|| errno!(ENOENT))?;
		let mountpoint = mountpoint_mutex.lock();

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let mut io = io_mutex.lock();

		// The filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let mut fs = fs_mutex.lock();

		let inode = fs.get_inode(&mut *io, Some(parent.get_location().get_inode()), &name)?;
		let mut file = fs.load_file(&mut *io, inode, name)?;

		if follow_links {
			if let FileContent::Link(link_path) = file.get_content() {
				let link_path = Path::from_str(link_path.as_bytes(), false)?;
				let new_path = parent.get_path()?.concat(&link_path)?;

				drop(fs);
				drop(io);
				drop(mountpoint);
				return self.get_file_from_path_(&new_path, uid, gid, follow_links, 1);
			}
		}

		file.set_parent_path(parent.get_path()?);
		update_location(&mut file, &mountpoint);
		Arc::new(Mutex::new(file))
	}

	// TODO Use the cache
	/// Creates a file, adds it to the VFS, then returns it. The file will be
	/// located into the directory `parent`.
	///
	/// If `parent` is not a directory, the function returns an error.
	///
	/// Arguments:
	/// - `name` is the name of the file.
	/// - `uid` is the id of the owner user.
	/// - `gid` is the id of the owner group.
	/// - `mode` is the permission of the file.
	/// - `content` is the content of the file. This value also determines the
	/// file type.
	pub fn create_file(
		&mut self,
		parent: &mut File,
		name: String,
		uid: Uid,
		mut gid: Gid,
		mode: Mode,
		content: FileContent,
	) -> Result<Arc<Mutex<File>>, Errno> {
		// If file already exist, error
		if self
			.get_file_from_parent(parent, name.try_clone()?, uid, gid, false)
			.is_ok()
		{
			return Err(errno!(EEXIST));
		}

		// Checking for errors
		if parent.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !parent.can_write(uid, gid) {
			return Err(errno!(EACCES));
		}

		// If SGID is set, the newly created file shall inherit the group ID of the
		// parent directory
		if parent.get_mode() & file::S_ISGID != 0 {
			gid = parent.get_gid();
		}

		// Getting the mountpoint
		let mountpoint_mutex = parent
			.get_location()
			.get_mountpoint()
			.ok_or_else(|| errno!(ENOENT))?;
		let mountpoint = mountpoint_mutex.lock();
		if mountpoint.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let mut io = io_mutex.lock();

		// Getting the filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let mut fs = fs_mutex.lock();
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}

		// The parent directory's inode
		let parent_inode = parent.get_location().get_inode();

		// Adding the file to the filesystem
		let mut file = fs.add_file(&mut *io, parent_inode, name, uid, gid, mode, content)?;

		// Adding the file to the parent's entries
		file.set_parent_path(parent.get_path()?);
		parent.add_entry(file.get_name().try_clone()?, file.to_dir_entry())?;

		drop(fs);
		update_location(&mut file, &mountpoint);
		Arc::new(Mutex::new(file))
	}

	/// Creates a new hard link.
	///
	/// Arguments:
	/// - `target` is the target file.
	/// - `parent` is the parent directory of the new link.
	/// - `name` is the name of the link.
	/// - `uid` is the id of the owner user.
	/// - `gid` is the id of the owner group.
	pub fn create_link(
		&mut self,
		target: &mut File,
		parent: &mut File,
		name: &[u8],
		uid: Uid,
		gid: Gid,
	) -> Result<(), Errno> {
		// Checking the parent file is a directory
		if parent.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !parent.can_write(uid, gid) {
			return Err(errno!(EACCES));
		}
		// Checking the target and source are both on the same mountpoint
		if target.get_location().get_mountpoint_id() != parent.get_location().get_mountpoint_id() {
			return Err(errno!(EXDEV));
		}

		// Getting the mountpoint
		let mountpoint_mutex = target
			.get_location()
			.get_mountpoint()
			.ok_or_else(|| errno!(ENOENT))?;
		let mountpoint = mountpoint_mutex.lock();
		if mountpoint.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let mut io = io_mutex.lock();

		// Getting the filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let mut fs = fs_mutex.lock();
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}

		fs.add_link(
			&mut *io,
			parent.get_location().get_inode(),
			name,
			target.get_location().get_inode(),
		)?;
		target.set_hard_links_count(target.get_hard_links_count() + 1);

		Ok(())
	}

	// TODO Use the cache
	/// Removes the file `file` from the VFS.
	///
	/// If the file doesn't exist, the function returns an error.
	///
	/// If the file is a non-empty directory, the function returns an error.
	///
	/// Arguments:
	/// - `uid` is the User ID of the user removing the file.
	/// - `gid` is the Group ID of the user removing the file.
	pub fn remove_file(&mut self, file: &File, uid: Uid, gid: Gid) -> Result<(), Errno> {
		if file.is_busy() {
			return Err(errno!(EBUSY));
		}

		// The parent directory.
		let parent_mutex = self.get_file_from_path(file.get_parent_path(), uid, gid, true)?;
		let parent = parent_mutex.lock();
		let parent_inode = parent.get_location().get_inode();

		// Checking permissions
		if !file.can_write(uid, gid) || !parent.can_write(uid, gid) {
			return Err(errno!(EACCES));
		}

		// Getting the mountpoint
		let location = file.get_location();
		let mountpoint_mutex = location.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
		let mountpoint = mountpoint_mutex.lock();
		if mountpoint.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Getting the IO interface
		let io_mutex = mountpoint.get_source().get_io()?;
		let mut io = io_mutex.lock();

		// Getting the filesystem
		let fs_mutex = mountpoint.get_filesystem();
		let mut fs = fs_mutex.lock();
		if fs.is_readonly() {
			return Err(errno!(EROFS));
		}

		// Removing the file
		fs.remove_file(&mut *io, parent_inode, file.get_name())?;

		if file.get_hard_links_count() > 1 {
			// If the file is a named pipe or socket, free its now unused buffer
			buffer::release(location);
		}

		Ok(())
	}

	/// Maps the page at offset `off` in the file at location `loc`.
	///
	/// On success, the function returns a reference to the page.
	///
	/// If the file doesn't exist, the function returns an error.
	pub fn map_file(&mut self, loc: FileLocation, off: usize) -> Result<NonNull<u8>, Errno> {
		// TODO if the page is being init, read from disk
		self.file_mappings_manager.map(loc, off)?;

		todo!();
	}

	/// Maps the page at offset `off` in the file at location `loc`.
	///
	/// If the page is not mapped, the function does nothing.
	pub fn unmap_file(&mut self, loc: &FileLocation, off: usize) {
		// TODO sync to disk if necessary
		self.file_mappings_manager.unmap(loc, off);
	}
}

/// The instance of the VFS.
static VFS: Mutex<Option<VFS>> = Mutex::new(None);

/// Returns a mutable reference to the VFS.
///
/// If the cache is not initialized, the Option is `None`.
///
/// If the function is called from a module, the VFS can be assumed to be initialized.
pub fn get() -> &'static Mutex<Option<VFS>> {
	&VFS
}