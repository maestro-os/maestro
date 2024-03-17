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

//! `*at` system calls allow to perform operations on files without having to redo the whole
//! path-resolution each time.
//!
//! This module implements utility functions for those system calls.

use crate::file::{
	fd::FileDescriptorTable,
	path::Path,
	vfs,
	vfs::{ResolutionSettings, Resolved},
	FileLocation,
};
use core::ffi::c_int;
use utils::{errno, errno::EResult};

/// Special value to be used as file descriptor, telling to take the path relative to the
/// current working directory.
pub const AT_FDCWD: c_int = -100;

/// Flag: If pathname is a symbolic link, do not dereference it: instead return
/// information about the link itself.
pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
/// Flag: Perform access checks using the effective user and group IDs.
pub const AT_EACCESS: c_int = 0x200;
/// Flag: If pathname is a symbolic link, dereference it.
pub const AT_SYMLINK_FOLLOW: c_int = 0x400;
/// Flag: Don't automount the terminal component of `pathname` if it is a directory that is an
/// automount point.
pub const AT_NO_AUTOMOUNT: c_int = 0x800;
/// Flag: If `pathname` is an empty string, operate on the file referred to by `dirfd`.
pub const AT_EMPTY_PATH: c_int = 0x1000;
/// Flag: Do whatever `stat` does.
pub const AT_STATX_SYNC_AS_STAT: c_int = 0x0000;
/// Flag: Force the attributes to be synchronized with the server.
pub const AT_STATX_FORCE_SYNC: c_int = 0x2000;
/// Flag: Don't synchronize anything, but rather take cached information.
pub const AT_STATX_DONT_SYNC: c_int = 0x4000;

/// Returns the location of the file pointed to by the given file descriptor.
///
/// Arguments:
/// - `fds` is the file descriptors table
/// - `fd` is the file descriptor
///
/// If the given file descriptor is invalid, the function returns [`errno::EBADF`].
fn fd_to_loc(fds: &FileDescriptorTable, fd: c_int) -> EResult<FileLocation> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}
	let open_file_mutex = fds
		.get_fd(fd as _)
		.ok_or(errno!(EBADF))?
		.get_open_file()
		.clone();
	let open_file = open_file_mutex.lock();
	Ok(open_file.get_location().clone())
}

/// Returns the file for the given path `path`.
///
/// Arguments:
/// - `fds` is the file descriptors table to use
/// - `rs` is the path resolution settings to use
/// - `dirfd` is the file descriptor of the parent directory
/// - `path` is the path relative to the parent directory
/// - `flags` is the set of `AT_*` flags
///
/// **Note**: the `start` field of [`ResolutionSettings`] is used as the current working
/// directory.
pub fn get_file<'p>(
	fds: &FileDescriptorTable,
	mut rs: ResolutionSettings,
	dirfd: c_int,
	path: &'p Path,
	flags: c_int,
) -> EResult<Resolved<'p>> {
	// Prepare resolution settings
	let follow_links = if rs.follow_link {
		flags & AT_SYMLINK_NOFOLLOW == 0
	} else {
		flags & AT_SYMLINK_FOLLOW != 0
	};
	rs.follow_link = follow_links;
	// If not starting from current directory, get location
	if dirfd != AT_FDCWD {
		rs.start = fd_to_loc(fds, dirfd)?;
	}
	if path.is_empty() {
		// Validation
		if flags & AT_EMPTY_PATH == 0 {
			return Err(errno!(ENOENT));
		}
		let file = vfs::get_file_from_location(&rs.start)?;
		Ok(Resolved::Found(file))
	} else {
		vfs::resolve_path(path, &rs)
	}
}
