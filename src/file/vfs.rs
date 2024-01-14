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

use crate::errno;
use crate::errno::EResult;
use crate::file::buffer;
use crate::file::mapping;
use crate::file::mountpoint;
use crate::file::open_file::OpenFile;
use crate::file::path::Path;
use crate::file::perm;
use crate::file::perm::AccessProfile;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::file::FileType;
use crate::file::Mode;
use crate::file::MountPoint;
use crate::limits;
use crate::util::container::string::String;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryClone;
use core::ptr::NonNull;

// TODO implement and use cache

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
pub fn get_file_by_location(location: &FileLocation) -> EResult<Arc<Mutex<File>>> {
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

/// `follows_count` is the number of links that have been followed since the
/// beginning of the path resolution.
fn get_file_by_path_impl(
	path: &Path,
	ap: &AccessProfile,
	follow_links: bool,
	follows_count: usize,
) -> EResult<Arc<Mutex<File>>> {
	let path = Path::root().concat(path)?;

	// Get the path's deepest mountpoint
	let mountpoint_mutex = mountpoint::get_deepest(&path).ok_or_else(|| errno!(ENOENT))?;
	let mountpoint = mountpoint_mutex.lock();
	let mountpath = mountpoint.get_path();

	// Get the IO interface
	let io_mutex = mountpoint.get_source().get_io()?;
	let mut io = io_mutex.lock();

	// Get the path of the file beginning from the start of its filesystem
	let inner_path = path.range_from(mountpoint.get_path().get_elements_count()..)?;

	// The filesystem
	let fs_mutex = mountpoint.get_filesystem();
	let mut fs = fs_mutex.lock();

	// The root inode
	let mut inode = fs.get_root_inode(&mut *io)?;
	let mut file = fs.load_file(&mut *io, inode, String::new())?;

	for i in 0..inner_path.get_elements_count() {
		inode = fs.get_inode(&mut *io, Some(inode), &inner_path[i])?;

		// Check permissions
		if i < inner_path.get_elements_count() - 1 && !ap.can_search_directory(&file) {
			return Err(errno!(EACCES));
		}
		// Get file
		file = fs.load_file(&mut *io, inode, inner_path[i].try_clone()?)?;

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
				return get_file_by_path_impl(&new_path, ap, follow_links, follows_count + 1);
			}
		}
	}

	let mut parent_path = path;
	parent_path.pop();
	file.set_parent_path(parent_path);

	drop(fs);

	update_location(&mut file, &mountpoint);
	let file = Arc::new(Mutex::new(file))?;
	Ok(file)
}

// TODO Add a param to choose between the mountpoint and the fs root?
/// Returns a reference to the file at path `path`.
///
/// If the file doesn't exist, the function returns an error.
///
/// If the path is relative, the function starts from the root.
///
/// Arguments:
/// - `ap` is the access profile to check permissions
/// - `follow_links` is `true`, the function follows symbolic links
pub fn get_file_from_path(
	path: &Path,
	ap: &AccessProfile,
	follow_links: bool,
) -> EResult<Arc<Mutex<File>>> {
	get_file_by_path_impl(path, ap, follow_links, 0)
}

/// Returns a reference to the file `name` located in the directory `parent`.
///
/// If the file doesn't exist, the function returns an error.
///
/// Arguments:
/// - `parent` is the parent directory
/// - `name` is the name of the file
/// - `ap` is the access profile to check permissions
/// - `follow_links` is `true`, the function follows symbolic links
pub fn get_file_from_parent(
	parent: &File,
	name: String,
	ap: &AccessProfile,
	follow_links: bool,
) -> EResult<Arc<Mutex<File>>> {
	// Check for errors
	if parent.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_search_directory(parent) {
		return Err(errno!(EACCES));
	}

	// Get the path's deepest mountpoint
	let mountpoint_mutex = parent
		.get_location()
		.get_mountpoint()
		.ok_or_else(|| errno!(ENOENT))?;
	let mountpoint = mountpoint_mutex.lock();

	// Get the IO interface
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
			return get_file_by_path_impl(&new_path, ap, follow_links, 1);
		}
	}

	file.set_parent_path(parent.get_path()?);
	update_location(&mut file, &mountpoint);

	Ok(Arc::new(Mutex::new(file))?)
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
