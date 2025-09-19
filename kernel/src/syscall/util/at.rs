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

use crate::file::{
	fd::fd_to_file,
	vfs,
	vfs::{ResolutionSettings, Resolved},
};
use core::ffi::c_int;
use utils::{collections::path::Path, errno, errno::EResult};

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

/// Returns the file for the given path `path`.
///
/// Arguments:
/// - `dirfd` is the file descriptor of the parent directory
/// - `path` is the path relative to the parent directory
/// - `flags` is the set of `AT_*` flags
/// - `create`: if `true`, the function might return [`Resolved::Creatable`] if the file does not
///   exist
/// - `follow_link` if `true`, links are followed (unless bypassed by a flag)
pub fn get_file(
	dirfd: c_int,
	path: Option<&Path>,
	flags: c_int,
	create: bool,
	follow_link: bool,
) -> EResult<Resolved> {
	let mut rs = ResolutionSettings::cur_task(create, follow_link);
	// If not starting from current directory, get location
	if dirfd != AT_FDCWD {
		let cwd = fd_to_file(dirfd)?
			.vfs_entry
			.clone()
			.ok_or_else(|| errno!(ENOTDIR))?;
		rs.cwd = Some(cwd);
	}
	match path {
		Some(path) if !path.is_empty() => {
			let follow_link = if rs.follow_link {
				flags & AT_SYMLINK_NOFOLLOW == 0
			} else {
				flags & AT_SYMLINK_FOLLOW != 0
			};
			rs.follow_link = follow_link;
			vfs::resolve_path(path, &rs)
		}
		// Empty path
		Some(_) => {
			// Validation
			if flags & AT_EMPTY_PATH == 0 {
				return Err(errno!(ENOENT));
			}
			Ok(Resolved::Found(rs.cwd.unwrap()))
		}
		None => {
			// Validation
			if dirfd == AT_FDCWD {
				return Err(errno!(EFAULT));
			}
			Ok(Resolved::Found(rs.cwd.unwrap()))
		}
	}
}
