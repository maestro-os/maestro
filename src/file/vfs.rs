//! The VFS (Virtual FileSystem) is a entity which aggregates every mounted
//! filesystems into one. To manipulate files, the VFS should be used instead of
//! calling the filesystems' functions directly.

use super::pipe::PipeBuffer;
use super::socket::Socket;
use crate::errno;
use crate::errno::Errno;
use crate::file;
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
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::lock::IntMutex;
use crate::util::ptr::SharedPtr;
use crate::util::FailableClone;

/// Updates the location of the file `file` according to the given mountpoint
/// `mountpoint`.
///
/// If the file in not located on a filesystem, the function does nothing.
fn update_location(file: &mut File, mountpoint: &MountPoint) {
	match file.get_location_mut() {
		FileLocation::Filesystem {
			mountpoint_id,
			..
		} => *mountpoint_id = Some(mountpoint.get_id()),

		_ => {},
	}
}

/// The Virtual FileSystem.
///
/// This structure acts as an aggregator of every mounted filesystems, but also
/// as a cache to speedup file accesses.
pub struct VFS {
	// TODO Add files caching

	/// All the system's pipes. The key is the location of the file associated with the entry.
	pipes: HashMap<FileLocation, SharedPtr<PipeBuffer>>,
	/// All the system's sockets. The key is the location of the file associated with the entry.
	sockets: HashMap<FileLocation, SharedPtr<Socket>>,
}

impl VFS {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			pipes: HashMap::new(),
			sockets: HashMap::new(),
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
		location: &FileLocation
	) -> Result<SharedPtr<File>, Errno> {
		match location {
			FileLocation::Filesystem {
				inode,
				..
			} => {
				// Getting the mountpoint
				let mountpoint_mutex = location.get_mountpoint()
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

				let file = fs.load_file(io, *inode, String::new())?;
				SharedPtr::new(file)
			},

			FileLocation::Virtual { id: _ } => {
				// TODO
				todo!();
			},
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
	) -> Result<SharedPtr<File>, Errno> {
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
		if !file.can_execute(uid, gid) {
			return Err(errno!(EACCES));
		}

		for i in 0..inner_path.get_elements_count() {
			inode = fs.get_inode(io, Some(inode), &inner_path[i])?;

			// Checking permissions
			file = fs.load_file(io, inode, inner_path[i].failable_clone()?)?;
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

					drop(fs_guard);
					drop(io_guard);
					drop(mountpoint_guard);
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

		let mut parent_path = path.failable_clone()?;
		parent_path.pop();
		file.set_parent_path(parent_path);

		drop(fs_guard);
		update_location(&mut file, &mountpoint);
		SharedPtr::new(file)
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
	/// - `follow_links` is true, the function follows symbolic links.
	pub fn get_file_from_path(
		&mut self,
		path: &Path,
		uid: Uid,
		gid: Gid,
		follow_links: bool,
	) -> Result<SharedPtr<File>, Errno> {
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
	/// - `follow_links` is true, the function follows symbolic links.
	pub fn get_file_from_parent(
		&mut self,
		parent: &mut File,
		name: String,
		uid: Uid,
		gid: Gid,
		follow_links: bool,
	) -> Result<SharedPtr<File>, Errno> {
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

		let inode = fs.get_inode(io, Some(parent.get_location().get_inode()), &name)?;
		let mut file = fs.load_file(io, inode, name)?;

		if follow_links {
			if let FileContent::Link(link_path) = file.get_content() {
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
	) -> Result<SharedPtr<File>, Errno> {
		match self.get_file_from_parent(parent, name.failable_clone()?, uid, gid, false) {
			// If file already exist, error
			Ok(_) => return Err(errno!(EEXIST)),
			// If file doesn't exist, do nothing
			Err(_) => {}
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
		let parent_inode = parent.get_location().get_inode();

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
		name: String,
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

		fs.add_link(
			io,
			parent.get_location().get_inode(),
			&name,
			target.get_location().get_inode(),
		)
		// TODO Update file
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
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get();
		let parent_inode = parent.get_location().get_inode();

		// Checking permissions
		if !file.can_write(uid, gid) || !parent.can_write(uid, gid) {
			return Err(errno!(EACCES));
		}

		// Getting the mountpoint
		let location = file.get_location();
		let mountpoint_mutex = location.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
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

		if file.get_hard_links_count() > 1 {
			// If the file is a named pipe or socket, free its now unused buffer
			self.free_location(location);
		}

		Ok(())
	}

	/// Allocates a virtual location.
	///
	/// If every possible locations are used (unlikely), the function returns an error.
	///
	/// When the file associated with the location is removed, the location is freed automaticaly.
	pub fn alloc_virt_location(&mut self) -> Result<FileLocation, Errno> {
		// TODO
		todo!();
	}

	/// Frees the given file location and its associated pipe or socket.
	///
	/// If the location doesn't exist, the function does nothing.
	pub fn free_location(&mut self, loc: &FileLocation) {
		let _ = self.pipes.remove(loc);
		let _ = self.sockets.remove(loc);

		// TODO free location
		todo!();
	}

	/// Returns the pipe associated with the file at location `loc`.
	///
	/// If the pipe doesn't exist, the function creates it.
	pub fn get_fifo(&mut self, loc: &FileLocation) -> Result<SharedPtr<PipeBuffer>, Errno> {
		match self.pipes.get(loc) {
			Some(buff) => Ok(buff.clone()),

			None => {
				// The pipe buffer doesn't exist, create it
				let buff = SharedPtr::new(PipeBuffer::new()?)?;
				self.pipes.insert(loc.clone(), buff.clone())?;

				Ok(buff)
			},
		}
	}

	/// Returns the socket associated with the file at location `loc`.
	///
	/// If the socket doesn't exist, the function creates it.
	pub fn get_socket(&mut self, loc: &FileLocation) -> Result<SharedPtr<PipeBuffer>, Errno> {
		match self.pipes.get(loc) {
			Some(buff) => Ok(buff.clone()),

			None => {
				// The socket buffer doesn't exist, create it
				/*let buff = SharedPtr::new(Socket::new()?)?;
				self.sockets.insert(loc.clone(), buff.clone())?;

				Ok(buff)*/

				// TODO
				todo!();
			},
		}
	}
}

/// The instance of the VFS.
static VFS: IntMutex<Option<VFS>> = IntMutex::new(None);

/// Returns a mutable reference to the VFS.
/// If the cache is not initialized, the Option is None. If the function is
/// called from a module, the VFS can be assumed to be initialized.
pub fn get() -> &'static IntMutex<Option<VFS>> {
	&VFS
}
