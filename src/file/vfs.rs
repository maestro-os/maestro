//! The VFS (Virtual FileSystem) is a entity which aggregates every mounted
//! filesystems into one.
//!
//! To manipulate files, the VFS should be used instead of
//! calling the filesystems' functions directly.

use crate::errno;
use crate::errno::EResult;
use crate::file::buffer;
use crate::file::mapping;
use crate::file::mountpoint;
use crate::file::open_file::OpenFile;
use crate::file::path::{Component, Path};
use crate::file::perm;
use crate::file::perm::AccessProfile;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::file::FileType;
use crate::file::Mode;
use crate::file::MountPoint;
use crate::process::Process;
use crate::util::container::string::String;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryClone;
use core::ptr::NonNull;

// TODO implement and use cache

/// The start position for a path resolution operation.
///
/// **Note**: if the path to resolve is absolute, this data is ignored.
pub enum ResolutionStart<'s> {
	/// Start resolution from the given path. This is usually the current working directory of the
	/// process.
	Path(&'s Path),
	/// Start resolution from the given location. This is usually the `fd` argument in `*at`-style
	/// system calls.
	///
	/// This variant overrides the root location.
	Location(FileLocation),
}

impl<'s> Default for ResolutionStart<'s> {
	/// Start from the root path.
	fn default() -> Self {
		Self::Path(Path::root())
	}
}

/// Settings for a path resolution operation.
pub struct ResolutionSettings<'s> {
	/// The location of the root directory for the operation.
	///
	/// Contrary to the `start` field, resolution *cannot* access a parent of this path.
	pub root: FileLocation,
	/// The beginning position of the path resolution.
	pub start: ResolutionStart<'s>,

	/// The access profile to use for resolution.
	pub access_profile: &'s AccessProfile,

	/// If `true`, the path is resolved for creation, meaning the operation will not fail if the
	/// file does not exist.
	pub create: bool,
	/// If `true`, path resolution follows symbolic links.
	pub follow_links: bool,
}

impl<'s> Default for ResolutionSettings<'s> {
	/// Returns the default settings **for kernel access**.
	///
	/// The resolution starts from the root of the VFS, and symbolic links are followed.
	fn default() -> Self {
		Self {
			root: FileLocation::root(),
			start: ResolutionStart::default(),

			access_profile: &AccessProfile::KERNEL,

			create: false,
			follow_links: true,
		}
	}
}

impl<'s> ResolutionSettings<'s> {
	/// Kernel access, without following symbolic links.
	pub const fn kernel_nofollow() -> Self {
		Self {
			follow_links: false,
			..Default::default()
		}
	}

	/// Returns simple settings, specifying only the `access_profile` and whether symbolic links
	/// should be followed.
	pub const fn simple(access_profile: &'s AccessProfile, follow_links: bool) -> Self {
		Self {
			access_profile,
			follow_links,
			..Default::default()
		}
	}

	/// Returns the default for the given process.
	///
	/// `follow_links` tells whether symbolic links are followed.
	pub fn for_process(proc: &'s Process, follow_links: bool) -> Self {
		Self {
			root: proc.chroot.clone(),
			start: ResolutionStart::Path(&proc.cwd),
			access_profile: &proc.access_profile,
			follow_links,
			..Default::default()
		}
	}
}

/// The resolute of the path resolution operation.
pub enum Resolved<'s> {
	/// The file has been found.
	Found(Arc<Mutex<File>>),
	/// The file can be created.
	///
	/// This variant can be returned only if the `create` field is set to `true` in
	/// [`ResolutionSettings`].
	Creatable {
		/// The parent directory in which the file is to be created.
		parent: Arc<Mutex<File>>,
		/// The name of the file to be created.
		name: &'s [u8],
	},
}

/// Like [`get_file_from_path`], but returns `None` is the file does not exist.
pub fn get_file_from_path_opt(
	path: &Path,
	resolution_settings: &ResolutionSettings,
) -> EResult<Option<Arc<Mutex<File>>>> {
	let file = match resolve_path(path, resolution_settings)? {
		Resolved::Found(file) => Some(file),
		_ => None,
	};
	Ok(file)
}

/// Returns the file at the given `path`.
///
/// If the file does not exist, the function returns [`ENOENT`].
pub fn get_file_from_path(
	path: &Path,
	resolution_settings: &ResolutionSettings,
) -> EResult<Arc<Mutex<File>>> {
	get_file_from_path_opt(path, resolution_settings)?.ok_or_else(|| errno!(ENOENT))
}

/// Resolves the given `path` with the given `settings`.
///
/// The following conditions can cause errors:
/// - If the path is empty, the function returns [`errno::ENOMEM`].
/// - If a component of the path cannot be accessed with the provided access profile, the function
///   returns [`errno::EACCES`].
/// - If a component of the path (excluding the last) is not a directory nor a symbolic link, the
///   function returns [`errno::ENOTDIR`].
/// - If a component of the path (excluding the last) is a symbolic link and following them is
///   disabled, the function returns [`errno::ENOTDIR`].
/// - If the resolution of the path requires more symbolic link indirections than
///   [`limits::SYMLOOP_MAX`], the function returns [`errno::ELOOP`].
pub fn resolve_path<'p>(path: &'p Path, settings: &ResolutionSettings) -> EResult<Resolved<'p>> {
	// Required by POSIX
	if path.is_empty() {
		return Err(errno!(ENOENT));
	}

	// Get start file
	let start = if path.is_absolute() {
		&settings.root
	} else {
		match &settings.start {
			ResolutionStart::Path(path) => {
				// TODO chain paths?
				todo!()
			}
			ResolutionStart::Location(loc) => loc,
		}
	};
	let mut file_mutex = get_file_from_location(start)?;

	// Iterate on components
	let mut iter = path.components().peekable();
	for comp in &mut iter {
		let name = match comp {
			Component::ParentDir => b"..",
			Component::Normal(name) => name,
			// Ignore
			Component::RootDir | Component::CurDir => continue,
		};
		let file = file_mutex.lock();
		match file.get_content() {
			FileContent::Directory(entries) => {
				// Check permission
				if !settings.access_profile.can_search_directory(&file) {
					return Err(errno!(EACCES));
				}
				let Some(entry) = entries.get(name) else {
					// If this is the last component
					let is_last = iter.peek().is_none();
					// If the last component does not exist and the file may be created
					let res = if is_last && settings.create {
						Ok(Resolved::Creatable {
							parent: file_mutex,
							name,
						})
					} else {
						Err(errno!(ENOENT))
					};
					return res;
				};
				let mountpoint_id = file
					.location
					.get_mountpoint_id()
					.ok_or_else(|| errno!(ENOENT))?;
				// The location on the current filesystem
				let mut loc = FileLocation::Filesystem {
					mountpoint_id,
					inode: entry.inode,
				};
				// Update location if on a different mountpoint
				if let Some(mountpoint) = mountpoint::from_location(&loc) {
					loc = mountpoint.lock().get_target_location().clone();
				}
				file_mutex = get_file_from_location(&loc)?;
			}
			// Follow link, if enabled
			FileContent::Link(link_path) if settings.follow_links => {
				// TODO resolve link
				todo!()
			}
			_ => return Err(errno!(ENOTDIR)),
		}
	}

	Ok(Resolved::Found(file_mutex))
}

/// Updates the location of the file `file` according to the given mountpoint
/// `mountpoint`.
///
/// If the file in not located on a filesystem, the function does nothing.
fn update_location(file: &mut File, mountpoint: &MountPoint) {
	if let FileLocation::Filesystem {
		mountpoint_id, ..
	} = &mut file.location
	{
		*mountpoint_id = mountpoint.get_id();
	}
}

/// Returns the file corresponding to the given location `location`.
///
/// This function doesn't set the name of the file since it cannot be known solely on its
/// location.
///
/// If the file doesn't exist, the function returns an error.
pub fn get_file_from_location(location: &FileLocation) -> EResult<Arc<Mutex<File>>> {
	match location {
		FileLocation::Filesystem {
			inode, ..
		} => {
			// Get the mountpoint
			let mountpoint_mutex = location.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
			let mountpoint = mountpoint_mutex.lock();

			// Get the IO interface
			let io_mutex = mountpoint.get_source().get_io()?;
			let mut io = io_mutex.lock();

			// Get the filesystem
			let fs_mutex = mountpoint.get_filesystem();
			let mut fs = fs_mutex.lock();

			let mut file = fs.load_file(&mut *io, *inode, String::new())?;
			update_location(&mut file, &mountpoint);

			Ok(Arc::new(Mutex::new(file))?)
		}

		FileLocation::Virtual {
			id,
		} => {
			let name = crate::format!("virtual:{id}")?;
			let content = FileContent::Fifo; // TODO

			let file = Arc::new(Mutex::new(File::new(
				name,
				0, // TODO
				0, // TODO
				0o666,
				location.clone(),
				content,
			)?))?;
			Ok(file)
		}
	}
}

/// Creates a file, adds it to the VFS, then returns it. The file will be
/// located into the directory `parent`.
///
/// If `parent` is not a directory, the function returns an error.
///
/// Arguments:
/// - `name` is the name of the file
/// - `ap` is access profile to check permissions. This also determines the UID and GID to be used
/// for the created file
/// - `mode` is the permission of the file
/// - `content` is the content of the file. This value also determines the
/// file type
pub fn create_file(
	parent: &mut File,
	name: String,
	ap: &AccessProfile,
	mode: Mode,
	content: FileContent,
) -> EResult<Arc<Mutex<File>>> {
	// If file already exist, error
	if get_file_from_parent(parent, name.try_clone()?, ap, false).is_ok() {
		return Err(errno!(EEXIST));
	}

	// Check for errors
	if parent.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_write_directory(parent) {
		return Err(errno!(EACCES));
	}

	let uid = ap.get_euid();
	let gid = if parent.get_mode() & perm::S_ISGID != 0 {
		// If SGID is set, the newly created file shall inherit the group ID of the
		// parent directory
		parent.get_gid()
	} else {
		ap.get_egid()
	};

	// Get the mountpoint
	let mountpoint_mutex = parent
		.get_location()
		.get_mountpoint()
		.ok_or_else(|| errno!(ENOENT))?;
	let mountpoint = mountpoint_mutex.lock();
	if mountpoint.is_readonly() {
		return Err(errno!(EROFS));
	}

	// Get the IO interface
	let io_mutex = mountpoint.get_source().get_io()?;
	let mut io = io_mutex.lock();

	// Get the filesystem
	let fs_mutex = mountpoint.get_filesystem();
	let mut fs = fs_mutex.lock();
	if fs.is_readonly() {
		return Err(errno!(EROFS));
	}

	// Add the file to the filesystem
	let parent_inode = parent.get_location().get_inode();
	let mut file = fs.add_file(&mut *io, parent_inode, name, uid, gid, mode, content)?;

	// Add the file to the parent's entries
	file.set_parent_path(parent.get_path()?);
	parent.add_entry(file.get_name().try_clone()?, file.as_dir_entry())?;

	drop(fs);
	update_location(&mut file, &mountpoint);
	Ok(Arc::new(Mutex::new(file))?)
}

/// Creates a new hard link.
///
/// Arguments:
/// - `target` is the target file
/// - `parent` is the parent directory of the new link
/// - `name` is the name of the link
/// - `ap` is the access profile to check permissions
pub fn create_link(
	target: &mut File,
	parent: &File,
	name: &[u8],
	ap: &AccessProfile,
) -> EResult<()> {
	// Check the parent file is a directory
	if parent.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_write_directory(parent) {
		return Err(errno!(EACCES));
	}
	// Check the target and source are both on the same mountpoint
	if target.get_location().get_mountpoint_id() != parent.get_location().get_mountpoint_id() {
		return Err(errno!(EXDEV));
	}

	// Get the mountpoint
	let mountpoint_mutex = target
		.get_location()
		.get_mountpoint()
		.ok_or_else(|| errno!(ENOENT))?;
	let mountpoint = mountpoint_mutex.lock();
	if mountpoint.is_readonly() {
		return Err(errno!(EROFS));
	}

	// Get the IO interface
	let io_mutex = mountpoint.get_source().get_io()?;
	let mut io = io_mutex.lock();

	// Get the filesystem
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

/// Removes the file `file` from the VFS.
///
/// `ap` is the access profile to check permissions
///
/// If the file doesn't exist, the function returns an error.
///
/// If the file is a non-empty directory, the function returns an error.
pub fn remove_file(file: &mut File, ap: &AccessProfile) -> EResult<()> {
	// The parent directory
	let parent_mutex = get_file_from_path(file.get_parent_path(), ap, true)?;
	let parent = parent_mutex.lock();
	let parent_location = parent.get_location();

	// Check permissions
	if !ap.can_write_file(file) || !ap.can_write_directory(&parent) {
		return Err(errno!(EACCES));
	}

	// Defer remove if the file is in use
	let last_link = file.get_hard_links_count() == 1;
	let symlink = matches!(file.get_type(), FileType::Link);
	if last_link && !symlink && OpenFile::is_open(&file.location) {
		file.defer_remove();
		return Ok(());
	}

	let location = file.get_location();
	let name = file.get_name();

	// FIXME: what if the file and its parent are not on the same filesystem?
	// Get the mountpoint
	let mountpoint_mutex = location.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
	let mountpoint = mountpoint_mutex.lock();
	if mountpoint.is_readonly() {
		return Err(errno!(EROFS));
	}

	// Get the IO interface
	let io_mutex = mountpoint.get_source().get_io()?;
	let mut io = io_mutex.lock();

	// Get the filesystem
	let fs_mutex = mountpoint.get_filesystem();
	let mut fs = fs_mutex.lock();
	if fs.is_readonly() {
		return Err(errno!(EROFS));
	}

	// Remove the file
	let links_left = fs.remove_file(&mut *io, parent_location.get_inode(), name)?;
	if links_left == 0 {
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
pub fn map_file(loc: FileLocation, off: usize) -> EResult<NonNull<u8>> {
	// TODO if the page is being init, read from disk
	mapping::map(loc, off)?;

	todo!();
}

/// Maps the page at offset `off` in the file at location `loc`.
///
/// If the page is not mapped, the function does nothing.
pub fn unmap_file(loc: &FileLocation, off: usize) {
	// TODO sync to disk if necessary
	mapping::unmap(loc, off);
}
