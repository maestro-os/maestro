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

//! The VFS (Virtual FileSystem) is a entity which aggregates every mounted
//! filesystems into one.
//!
//! To manipulate files, the VFS should be used instead of
//! calling the filesystems' functions directly.

use super::{
	buffer,
	fs::Filesystem,
	mapping, mountpoint,
	open_file::OpenFile,
	path::{Component, Path},
	perm,
	perm::{AccessProfile, S_ISVTX},
	DeferredRemove, File, FileContent, FileLocation, FileType, INode, Mode, MountPoint,
};
use crate::{limits, process::Process};
use core::ptr::NonNull;
use utils::{
	collections::string::String, errno, errno::EResult, format, io::IO, lock::Mutex,
	ptr::arc::Arc, TryClone,
};

// TODO implement and use cache

/// Helper function for filesystem I/O. Provides mountpoint, I/O interface and filesystem handle
/// for the given location.
///
/// If `write` is set to `true`, the function checks the filesystem is not mounted in read-only. If
/// mounted in read-only, the function returns [`errno::EROFS`].
fn op<F, R>(loc: &FileLocation, write: bool, f: F) -> EResult<R>
where
	F: FnOnce(&MountPoint, &mut dyn IO, &mut dyn Filesystem) -> EResult<R>,
{
	// Get the mountpoint
	let mp_mutex = loc.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
	let mp = mp_mutex.lock();
	if write && mp.is_readonly() {
		return Err(errno!(EROFS));
	}

	// Get the IO interface
	let io_mutex = mp.get_source().get_io()?;
	let mut io = io_mutex.lock();

	// Get the filesystem
	let fs_mutex = mp.get_filesystem();
	let mut fs = fs_mutex.lock();
	if write && fs.is_readonly() {
		return Err(errno!(EROFS));
	}

	f(&mp, &mut *io, &mut *fs)
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
			let mp_mutex = location.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
			let mp = mp_mutex.lock();

			// Get the IO interface
			let io_mutex = mp.get_source().get_io()?;
			let mut io = io_mutex.lock();

			// Get the filesystem
			let fs_mutex = mp.get_filesystem();
			let mut fs = fs_mutex.lock();

			let mut file = fs.load_file(&mut *io, *inode, String::new())?;
			update_location(&mut file, &mp);

			Ok(Arc::new(Mutex::new(file))?)
		}

		FileLocation::Virtual {
			id,
		} => {
			let name = format!("virtual:{id}")?;
			let content = FileContent::Fifo; // TODO support sockets

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

/// Settings for a path resolution operation.
#[derive(Clone, Debug)]
pub struct ResolutionSettings {
	/// The location of the root directory for the operation.
	///
	/// Contrary to the `start` field, resolution *cannot* access a parent of this path.
	pub root: FileLocation,
	/// The beginning position of the path resolution.
	pub start: FileLocation,

	/// The access profile to use for resolution.
	pub access_profile: AccessProfile,

	/// If `true`, the path is resolved for creation, meaning the operation will not fail if the
	/// file does not exist.
	pub create: bool,
	/// If `true` and if the last component of the path is a symbolic link, path resolution
	/// follows it.
	pub follow_link: bool,
}

impl ResolutionSettings {
	/// Kernel access, following symbolic links.
	pub fn kernel_follow() -> Self {
		Self {
			root: mountpoint::root_location(),
			start: mountpoint::root_location(),

			access_profile: AccessProfile::KERNEL,

			create: false,
			follow_link: true,
		}
	}

	/// Kernel access, without following symbolic links.
	pub fn kernel_nofollow() -> Self {
		Self {
			follow_link: false,
			..Self::kernel_follow()
		}
	}

	/// Returns the default for the given process.
	///
	/// `follow_links` tells whether symbolic links are followed.
	pub fn for_process(proc: &Process, follow_links: bool) -> Self {
		Self {
			root: proc.chroot.clone(),
			start: proc.cwd.1.clone(),

			access_profile: proc.access_profile,

			create: false,
			follow_link: follow_links,
		}
	}
}

/// The resolute of the path resolution operation.
#[derive(Debug)]
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

/// Implementation of [`resolve_path`].
///
/// `symlink_rec` is the number of recursions due to symbolic links resolution.
fn resolve_path_impl<'p>(
	path: &'p Path,
	settings: &ResolutionSettings,
	symlink_rec: usize,
) -> EResult<Resolved<'p>> {
	// Get start file
	let start = if path.is_absolute() {
		&settings.root
	} else {
		&settings.start
	};
	let mut file_mutex = get_file_from_location(start)?;
	// Iterate on components
	let mut iter = path.components().peekable();
	while let Some(comp) = iter.next() {
		// If this is the last component
		let is_last = iter.peek().is_none();
		let file = file_mutex.lock();
		// Get the name of the next entry
		let name = match comp {
			Component::ParentDir if file.location != settings.root => b"..",
			Component::Normal(name) => name,
			// Ignore
			_ => continue,
		};
		let next_file = match file.get_content() {
			FileContent::Directory(entries) => {
				// Check permission
				if !settings.access_profile.can_search_directory(&file) {
					return Err(errno!(EACCES));
				}
				let Some(entry) = entries.get(name) else {
					// If the last component does not exist and if the file may be created
					let res = if is_last && settings.create {
						drop(file);
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
				// Update location if on a different filesystem
				if let Some(mp) = mountpoint::from_location(&loc) {
					let mp = mp.lock();
					let fs_mutex = mp.get_filesystem();
					let fs = fs_mutex.lock();
					loc = FileLocation::Filesystem {
						mountpoint_id: mp.get_id(),
						inode: fs.get_root_inode(),
					};
				}
				get_file_from_location(&loc)?
			}
			// Follow link, if enabled
			FileContent::Link(link_path) if !is_last || settings.follow_link => {
				// If too many recursions occur, error
				if symlink_rec + 1 > limits::SYMLOOP_MAX {
					return Err(errno!(ELOOP));
				}
				// Resolve link
				let rs = ResolutionSettings {
					root: settings.root.clone(),
					start: file.location.clone(),
					access_profile: settings.access_profile,
					create: false,
					follow_link: true,
				};
				let resolved = resolve_path_impl(link_path, &rs, symlink_rec + 1)?;
				let Resolved::Found(next_file) = resolved else {
					// Because `create` is set to `false`
					unreachable!();
				};
				next_file
			}
			_ => return Err(errno!(ENOTDIR)),
		};
		drop(file);
		file_mutex = next_file;
	}
	Ok(Resolved::Found(file_mutex))
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
	resolve_path_impl(path, settings, 0)
}

/// Updates the location of the file `file` according to the given mountpoint
/// `mountpoint`.
///
/// If the file in not located on a filesystem, the function does nothing.
fn update_location(file: &mut File, mp: &MountPoint) {
	if let FileLocation::Filesystem {
		mountpoint_id, ..
	} = &mut file.location
	{
		*mountpoint_id = mp.get_id();
	}
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
/// If the file does not exist, the function returns [`errno::ENOENT`].
pub fn get_file_from_path(
	path: &Path,
	resolution_settings: &ResolutionSettings,
) -> EResult<Arc<Mutex<File>>> {
	get_file_from_path_opt(path, resolution_settings)?.ok_or_else(|| errno!(ENOENT))
}

/// Creates a file, adds it to the VFS, then returns it.
///
/// Arguments:
/// - `parent` is the parent directory of the file to be created
/// - `name` is the name of the file to be created
/// - `ap` is access profile to check permissions. This also determines the UID and GID to be used
/// for the created file
/// - `mode` is the permissions of the file
/// - `content` is the content of the file. This value also determines the file type
///
/// The following errors can be returned:
/// - The filesystem is read-only: [`errno::EROFS`]
/// - I/O failed: [`errno::EIO`]
/// - Permissions to create the file are not fulfilled for the given `ap`: [`errno::EACCES`]
/// - `parent` is not a directory: [`errno::ENOTDIR`]
/// - The file already exists: [`errno::EEXIST`]
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn create_file(
	parent: &mut File,
	name: String,
	ap: &AccessProfile,
	mode: Mode,
	content: FileContent,
) -> EResult<Arc<Mutex<File>>> {
	// Validation
	if parent.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_write_directory(parent) {
		return Err(errno!(EACCES));
	}

	let parent_inode = parent.location.get_inode();
	let uid = ap.get_euid();
	let gid = if parent.get_mode() & perm::S_ISGID != 0 {
		// If SGID is set, the newly created file shall inherit the group ID of the
		// parent directory
		parent.get_gid()
	} else {
		ap.get_egid()
	};

	let mut file = op(&parent.location, true, |mp, io, fs| {
		let mut file = fs.add_file(&mut *io, parent_inode, name, uid, gid, mode, content)?;
		update_location(&mut file, mp);
		Ok(file)
	})?;
	// Add the file to the parent's entries
	file.set_parent_path(parent.get_path()?);
	parent.add_entry(file.get_name().try_clone()?, file.as_dir_entry())?;

	Ok(Arc::new(Mutex::new(file))?)
}

/// Creates a new hard link to the given target file.
///
/// Arguments:
/// - `parent` is the parent directory where the new link will be created
/// - `name` is the name of the link
/// - `target` is the target file
/// - `ap` is the access profile to check permissions
///
/// The following errors can be returned:
/// - The filesystem is read-only: [`errno::EROFS`]
/// - I/O failed: [`errno::EIO`]
/// - Permissions to create the link are not fulfilled for the given `ap`: [`errno::EACCES`]
/// - The number of links to the file is larger than [`limits::LINK_MAX`]: [`errno::EMLINK`]
/// - `target` is a directory: [`errno::EPERM`]
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn create_link(
	parent: &File,
	name: &[u8],
	target: &mut File,
	ap: &AccessProfile,
) -> EResult<()> {
	// Validation
	if parent.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	if target.get_type() == FileType::Directory {
		return Err(errno!(EPERM));
	}
	if target.get_hard_links_count() >= limits::LINK_MAX as u16 {
		return Err(errno!(EMLINK));
	}
	// Check the target and source are both on the same mountpoint
	if parent.location.get_mountpoint_id() != target.location.get_mountpoint_id() {
		return Err(errno!(EXDEV));
	}
	if !ap.can_write_directory(parent) {
		return Err(errno!(EACCES));
	}

	op(&target.location, true, |_mp, io, fs| {
		fs.add_link(
			&mut *io,
			parent.location.get_inode(),
			name,
			target.location.get_inode(),
		)
	})?;
	target.set_hard_links_count(target.get_hard_links_count() + 1);
	Ok(())
}

fn remove_file_impl(
	mp: &MountPoint,
	io: &mut dyn IO,
	fs: &mut dyn Filesystem,
	parent_inode: INode,
	name: &[u8],
) -> EResult<()> {
	let (links_left, inode) = fs.remove_file(&mut *io, parent_inode, name)?;
	if links_left == 0 {
		// If the file is a named pipe or socket, free its now unused buffer
		let loc = FileLocation::Filesystem {
			mountpoint_id: mp.get_id(),
			inode,
		};
		buffer::release(&loc);
	}
	Ok(())
}

/// Removes a file without checking permissions.
///
/// This is useful for deferred remove since permissions have already been checked before.
pub fn remove_file_unchecked(parent: &FileLocation, name: &[u8]) -> EResult<()> {
	op(parent, true, |mp, io, fs| {
		remove_file_impl(mp, io, fs, parent.get_inode(), name)
	})
}

/// Removes a file.
///
/// Arguments:
/// - `parent` is the parent directory of the file to remove
/// - `name` is the name of the file to remove
/// - `ap` is the access profile to check permissions
///
/// The following errors can be returned:
/// - The filesystem is read-only: [`errno::EROFS`]
/// - I/O failed: [`errno::EIO`]
/// - The file doesn't exist: [`errno::ENOENT`]
/// - Permissions to remove the file are not fulfilled for the given `ap`: [`errno::EACCES`]
/// - The file to remove is a mountpoint: [`errno::EBUSY`]
///
/// Other errors can be returned depending on the underlying filesystem.
pub fn remove_file(parent: &mut File, name: &[u8], ap: &AccessProfile) -> EResult<()> {
	// Check permission
	if !ap.can_write_directory(parent) {
		return Err(errno!(EACCES));
	}

	op(parent.get_location(), true, |mp, io, fs| {
		// Get the file
		let mut file = fs.load_file(&mut *io, parent.location.get_inode(), name.try_into()?)?;

		// Check permission
		let has_sticky_bit = parent.mode & S_ISVTX != 0;
		if has_sticky_bit && ap.get_euid() != file.get_uid() && ap.get_euid() != parent.get_uid() {
			return Err(errno!(EACCES));
		}
		// If the file to remove is a mountpoint, error
		if file.is_mountpoint() {
			return Err(errno!(EBUSY));
		}

		// Defer remove if the file is in use
		let last_link = file.get_hard_links_count() == 1;
		let symlink = file.get_type() == FileType::Link;
		let defer = last_link && !symlink && OpenFile::is_open(&file.location);

		if defer {
			file.defer_remove(DeferredRemove {
				parent: parent.location.clone(),
				name: name.try_into()?,
			});
		} else {
			remove_file_impl(mp, io, fs, parent.location.get_inode(), name)?;
		}
		Ok(())
	})
}

/// Helper function to remove a file from a given `path`.
pub fn remove_file_from_path(
	path: &Path,
	resolution_settings: &ResolutionSettings,
) -> EResult<()> {
	let file_name = path.file_name().ok_or_else(|| errno!(ENOENT))?;
	let parent = path.parent().ok_or_else(|| errno!(ENOENT))?;
	let parent = get_file_from_path(parent, resolution_settings)?;
	let mut parent = parent.lock();
	remove_file(&mut parent, file_name, &resolution_settings.access_profile)
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
