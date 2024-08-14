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

//! The VFS (Virtual FileSystem) is an entity which aggregates every mounted
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
	DeferredRemove, File, FileLocation, FileType, MountPoint, Stat,
};
use crate::{limits, process::Process};
use core::{intrinsics::unlikely, ptr::NonNull};
use utils::{errno, errno::EResult, lock::Mutex, ptr::arc::Arc};

// TODO implement and use cache

/// Helper function for filesystem I/O. Provides mountpoint, I/O interface and filesystem handle
/// for the given location.
///
/// If `write` is set to `true`, the function checks the filesystem is not mounted in read-only. If
/// mounted in read-only, the function returns [`errno::EROFS`].
#[inline]
fn op<F, R>(loc: &FileLocation, write: bool, f: F) -> EResult<R>
where
	F: FnOnce(&MountPoint, &dyn Filesystem) -> EResult<R>,
{
	// Get the mountpoint
	let mp_mutex = loc.get_mountpoint().ok_or_else(|| errno!(ENOENT))?;
	let mp = mp_mutex.lock();
	if write && unlikely(mp.is_readonly()) {
		return Err(errno!(EROFS));
	}
	// Get the filesystem
	let fs = mp.get_filesystem();
	if write && unlikely(fs.is_readonly()) {
		return Err(errno!(EROFS));
	}
	f(&mp, &*fs)
}

/// Returns the file corresponding to the given location `location`.
///
/// If the file doesn't exist, the function returns [`errno::ENOENT`].
pub fn get_file_from_location(location: FileLocation) -> EResult<Arc<Mutex<File>>> {
	let (ops, stat) = match location {
		FileLocation::Filesystem {
			inode, ..
		} => op(&location, false, |_, fs| {
			let ops = fs.node_from_inode(inode)?;
			let stat = ops.get_stat(inode, fs)?;
			Ok((Some(ops), stat))
		})?,
		loc @ FileLocation::Virtual(_) => {
			let buffer_mutex = buffer::get(&loc).ok_or_else(|| errno!(ENOENT))?;
			let stat = buffer_mutex.lock().get_stat()?;
			(None, stat)
		}
	};
	Ok(Arc::new(Mutex::new(File::new(location, ops, stat)))?)
}

/// Same as [`get_file_from_parent`], without checking permissions.
fn get_file_from_parent_unchecked(parent: &File, name: &[u8]) -> EResult<Arc<Mutex<File>>> {
	let parent_inode = parent.location.get_inode();
	let file = op(&parent.location, false, |mp, fs| {
		let (ent, ops) = parent
			.ops
			.entry_by_name(parent_inode, fs, name)?
			.ok_or_else(|| errno!(ENOENT))?;
		let inode = ent.inode;
		let stat = ops.get_stat(inode, fs)?;
		Ok(File::new(
			FileLocation::Filesystem {
				mountpoint_id: mp.get_id(),
				inode,
			},
			ops,
			stat,
		))
	})?;
	Ok(Arc::new(Mutex::new(file))?)
}

/// Returns the file with the `name` in the directory `parent`.
///
/// The function checks search access in the directory. If not allowed, the function returns
/// [`errno::EACCES`].
///
/// If the file doesn't exist, the function returns [`errno::ENOENT`].
pub fn get_file_from_parent(
	parent: &File,
	name: &[u8],
	ap: &AccessProfile,
) -> EResult<Arc<Mutex<File>>> {
	if !ap.can_search_directory(parent) {
		return Err(errno!(EACCES));
	}
	get_file_from_parent_unchecked(parent, name)
}

/// Same as [`get_file_from_parent`], but returns `None` if the file does not exist.
pub fn get_file_from_parent_opt(
	parent: &File,
	name: &[u8],
	ap: &AccessProfile,
) -> EResult<Option<Arc<Mutex<File>>>> {
	match get_file_from_parent(parent, name, ap) {
		Ok(f) => Ok(Some(f)),
		Err(e) if e.as_int() == errno::ENOENT => Ok(None),
		Err(e) => Err(e),
	}
}

/// Settings for a path resolution operation.
#[derive(Clone, Debug)]
pub struct ResolutionSettings {
	/// The location of the root directory for the operation.
	///
	/// Contrary to the `start` field, resolution *cannot* access a parent of this path.
	pub root: FileLocation,
	/// The beginning position of the path resolution. If `None`, resolution starts at root.
	pub start: Option<Arc<Mutex<File>>>,

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
			start: None,

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
			root: proc.chroot,
			start: Some(proc.cwd.1.clone()),

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

/// Resolves the symbolic link `link` and returns the target.
///
/// Arguments:
/// - `root` is the location of the root directory
/// - `lookup_dir` is the directory from which the resolution of the target starts
/// - `access_profile` is the access profile used for resolution
/// - `symlink_rec` is the number of recursions so far
///
/// Symbolic links are followed recursively, including the last element of the target path.
fn resolve_link(
	link: &mut File,
	root: FileLocation,
	lookup_dir: Arc<Mutex<File>>,
	access_profile: AccessProfile,
	symlink_rec: usize,
) -> EResult<Arc<Mutex<File>>> {
	// If too many recursions occur, error
	if symlink_rec + 1 > limits::SYMLOOP_MAX {
		return Err(errno!(ELOOP));
	}
	// Read link
	let link_path = link.read_link()?;
	// Resolve link
	let rs = ResolutionSettings {
		root,
		start: Some(lookup_dir),
		access_profile,
		create: false,
		follow_link: true,
	};
	let resolved = resolve_path_impl(&link_path, &rs, symlink_rec + 1)?;
	let Resolved::Found(target) = resolved else {
		// Because `create` is set to `false`
		unreachable!();
	};
	Ok(target)
}

/// Implementation of [`resolve_path`].
///
/// `symlink_rec` is the number of recursions due to symbolic links resolution.
fn resolve_path_impl<'p>(
	path: &'p Path,
	settings: &ResolutionSettings,
	symlink_rec: usize,
) -> EResult<Resolved<'p>> {
	// Get start lookup directory
	let mut lookup_dir = match (path.is_absolute(), &settings.start) {
		(false, Some(start)) => start.clone(),
		_ => get_file_from_location(settings.root)?,
	};
	let mut components = path.components();
	let Some(final_component) = components.next_back() else {
		return Ok(Resolved::Found(lookup_dir));
	};
	// Iterate on intermediate components
	for comp in components {
		// Get the name of the next entry
		let name = match comp {
			Component::ParentDir => b"..",
			Component::Normal(name) => name,
			// Ignore
			_ => continue,
		};
		// Search component
		let res = get_file_from_parent_opt(&lookup_dir.lock(), name, &settings.access_profile)?;
		let Some(subfile_mutex) = res else {
			return Err(errno!(ENOENT));
		};
		let mut subfile = subfile_mutex.lock();
		// If this is a mountpoint, continue resolution from the root of its filesystem
		if let Some(mp) = mountpoint::from_location(&subfile.location) {
			let loc = mp.lock().get_root_location();
			lookup_dir = get_file_from_location(loc)?;
			continue;
		}
		match subfile.stat.file_type {
			FileType::Directory => {
				drop(subfile);
				lookup_dir = subfile_mutex;
			}
			// Follow link, if enabled
			FileType::Link => {
				let target = resolve_link(
					&mut subfile,
					settings.root,
					lookup_dir,
					settings.access_profile,
					symlink_rec,
				)?;
				if target.lock().stat.file_type != FileType::Directory {
					return Err(errno!(ENOTDIR));
				}
				lookup_dir = target;
			}
			_ => return Err(errno!(ENOTDIR)),
		}
	}
	// Final component lookup
	let name = match final_component {
		Component::RootDir | Component::CurDir => {
			// If the component is `RootDir`, the entire path equals `/` and `lookup_dir` can only
			// be the root. If the component is `CurDir`, the `lookup_dir` is the target
			return Ok(Resolved::Found(lookup_dir));
		}
		Component::ParentDir => b"..",
		Component::Normal(name) => name,
	};
	let res = get_file_from_parent_opt(&lookup_dir.lock(), name, &settings.access_profile)?;
	let Some(file_mutex) = res else {
		// The file does not exist
		return if settings.create {
			Ok(Resolved::Creatable {
				parent: lookup_dir,
				name,
			})
		} else {
			Err(errno!(ENOENT))
		};
	};
	// The file exists
	let mut file = file_mutex.lock();
	// If the final file is a mountpoint, return the root to it
	if let Some(mp) = mountpoint::from_location(&file.location) {
		let loc = mp.lock().get_root_location();
		return Ok(Resolved::Found(get_file_from_location(loc)?));
	}
	// Resolve symbolic link if necessary
	if settings.follow_link && file.stat.file_type == FileType::Link {
		Ok(Resolved::Found(resolve_link(
			&mut file,
			settings.root,
			lookup_dir,
			settings.access_profile,
			symlink_rec,
		)?))
	} else {
		drop(file);
		Ok(Resolved::Found(file_mutex))
	}
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
	if settings.start.is_none() && path.is_empty() {
		return Err(errno!(ENOENT));
	}
	resolve_path_impl(path, settings, 0)
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
///   for the created file
/// - `stat` is the status of the newly created file
///
/// From the provided `stat`, the following fields are ignored:
/// - `nlink`
/// - `uid`
/// - `gid`
///
/// `uid` and `gid` are set according to `ap`.
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
	name: &[u8],
	ap: &AccessProfile,
	mut stat: Stat,
) -> EResult<Arc<Mutex<File>>> {
	// Validation
	if parent.stat.file_type != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_write_directory(parent) {
		return Err(errno!(EACCES));
	}
	stat.uid = ap.euid;
	let gid = if parent.stat.mode & perm::S_ISGID != 0 {
		// If SGID is set, the newly created file shall inherit the group ID of the
		// parent directory
		parent.stat.gid
	} else {
		ap.egid
	};
	stat.gid = gid;
	let parent_inode = parent.location.get_inode();
	let file = op(&parent.location, true, |mp, fs| {
		let (inode, ops) = parent.ops.add_file(parent_inode, fs, name, stat)?;
		let stat = ops.get_stat(inode, fs)?;
		Ok(File::new(
			FileLocation::Filesystem {
				mountpoint_id: mp.get_id(),
				inode,
			},
			ops,
			stat,
		))
	})?;
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
	if parent.stat.file_type != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	if target.stat.file_type == FileType::Directory {
		return Err(errno!(EPERM));
	}
	if target.stat.nlink >= limits::LINK_MAX as u16 {
		return Err(errno!(EMLINK));
	}
	if !ap.can_write_directory(parent) {
		return Err(errno!(EACCES));
	}
	// Check the target and source are both on the same mountpoint
	if parent.location.get_mountpoint_id() != target.location.get_mountpoint_id() {
		return Err(errno!(EXDEV));
	}
	op(&target.location, true, |_mp, fs| {
		parent
			.ops
			.link(&parent.location, name, target.location.get_inode())
	})?;
	target.stat.nlink += 1;
	Ok(())
}

/// Removes a file without checking permissions.
///
/// This is useful for deferred remove since permissions have already been checked before.
pub fn remove_file_unchecked(parent: &File, name: &[u8]) -> EResult<()> {
	op(&parent.location, true, |mp, fs| {
		let (links_left, inode) = parent.ops.unlink(&parent.location, name)?;
		if links_left == 0 {
			// If the file is a named pipe or socket, free its now unused buffer
			buffer::release(&FileLocation::Filesystem {
				mountpoint_id: mp.get_id(),
				inode,
			});
		}
		Ok(())
	})
}

/// Removes a file.
///
/// If the file is still open, the function defers removal until it is closed. For this reason, it
/// may retain a link to `parent`.
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
pub fn remove_file(parent: Arc<Mutex<File>>, name: &[u8], ap: &AccessProfile) -> EResult<()> {
	let parent_dir = parent.lock();
	// Check permission
	if !ap.can_write_directory(&parent_dir) {
		return Err(errno!(EACCES));
	}
	// Get file to remove
	let file_mutex = get_file_from_parent_unchecked(&parent_dir, name)?;
	let mut file = file_mutex.lock();
	// Check permission
	let has_sticky_bit = parent_dir.stat.mode & S_ISVTX != 0;
	if has_sticky_bit && ap.euid != file.stat.uid && ap.euid != parent_dir.stat.uid {
		return Err(errno!(EACCES));
	}
	// If the file to remove is a mountpoint, error
	if mountpoint::from_location(&file.location).is_some() {
		return Err(errno!(EBUSY));
	}
	// Defer remove if the file is in use. Else, remove it directly
	let last_link = file.stat.nlink == 1;
	let symlink = file.stat.file_type == FileType::Link;
	let defer = last_link && !symlink && OpenFile::is_open(&file.location);
	if defer {
		drop(parent_dir);
		file.defer_remove(DeferredRemove {
			parent,
			name: name.try_into()?,
		});
	} else {
		remove_file_unchecked(&parent_dir, name)?;
	}
	Ok(())
}

/// Helper function to remove a file from a given `path`.
pub fn remove_file_from_path(
	path: &Path,
	resolution_settings: &ResolutionSettings,
) -> EResult<()> {
	let file_name = path.file_name().ok_or_else(|| errno!(ENOENT))?;
	let parent = path.parent().ok_or_else(|| errno!(ENOENT))?;
	let parent = get_file_from_path(parent, resolution_settings)?;
	remove_file(parent, file_name, &resolution_settings.access_profile)
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
