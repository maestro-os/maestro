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

//! The `access` system call allows to check access to a given file.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// Special value, telling to take the path relative to the current working
/// directory.
pub const AT_FDCWD: i32 = -100;
/// If pathname is a symbolic link, do not dereference it: instead return
/// information about the link itself.
pub const AT_SYMLINK_NOFOLLOW: i32 = 0x100;
/// Perform access checks using the effective user and group IDs.
pub const AT_EACCESS: i32 = 0x200;
/// If pathname is a symbolic link, dereference it.
pub const AT_SYMLINK_FOLLOW: i32 = 0x400;
/// Don't automount the terminal component of `pathname` if it is a directory that is an automount
/// point.
pub const AT_NO_AUTOMOUNT: i32 = 0x800;
/// If `pathname` is an empty string, operate on the file referred to by `dirfd`.
pub const AT_EMPTY_PATH: i32 = 0x1000;
/// Do whatever `stat` does.
pub const AT_STATX_SYNC_AS_STAT: i32 = 0x0000;
/// Force the attributes to be synchronized with the server.
pub const AT_STATX_FORCE_SYNC: i32 = 0x2000;
/// Don't synchronize anything, but rather take cached informations.
pub const AT_STATX_DONT_SYNC: i32 = 0x4000;

/// Checks for existence of the file.
const F_OK: i32 = 0;
/// Checks the file can be read.
const R_OK: i32 = 4;
/// Checks the file can be written.
const W_OK: i32 = 2;
/// Checks the file can be executed.
const X_OK: i32 = 1;

/// Performs the access operation.
///
/// Arguments:
/// - `dirfd` is the file descriptor of the directory relative to which the check
/// is done.
/// - `pathname` is the path to the file.
/// - `mode` is a bitfield of access permissions to check.
/// - `flags` is a set of flags.
pub fn do_access(
	dirfd: Option<i32>,
	pathname: SyscallString,
	mode: i32,
	flags: Option<i32>,
) -> Result<i32, Errno> {
	let flags = flags.unwrap_or(0);
	let follow_symlinks = flags & AT_SYMLINK_NOFOLLOW == 0;
	// Use effective IDs instead of real IDs
	let eaccess = flags & AT_EACCESS != 0;

	let (path, cwd, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space_mutex = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space_mutex.lock();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EINVAL))?;
		let path = Path::from_str(pathname, true)?;

		let cwd = proc.cwd.clone();

		(path, cwd, proc.access_profile)
	};

	// Get file
	let mut path = path;
	if path.is_absolute() {
		// TODO
	} else if let Some(dirfd) = dirfd {
		if dirfd == AT_FDCWD {
			path = cwd.concat(&path)?;
		} else {
			// TODO Get file from fd and get its path to concat
			todo!();
		}
	}
	let file = vfs::get_file_from_path(&path, &ap, follow_symlinks)?;

	// Do access checks
	{
		let file = file.lock();
		if (mode & R_OK != 0) && !ap.check_read_access(&*file, eaccess) {
			return Err(errno!(EACCES));
		}
		if (mode & W_OK != 0) && !ap.check_write_access(&*file, eaccess) {
			return Err(errno!(EACCES));
		}
		if (mode & X_OK != 0) && !ap.check_execute_access(&*file, eaccess) {
			return Err(errno!(EACCES));
		}
	}

	Ok(0)
}

#[syscall]
pub fn access(pathname: SyscallString, mode: c_int) -> Result<i32, Errno> {
	do_access(None, pathname, mode, None)
}
