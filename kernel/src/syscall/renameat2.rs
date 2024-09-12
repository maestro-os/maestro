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

//! The `renameat2` allows to rename a file.

use super::util::at;
use crate::{
	file::{
		fd::FileDescriptorTable,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		FileType,
	},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

/// Flag: Don't replace new path if it exists. Return an error instead.
const RENAME_NOREPLACE: c_int = 1;
/// Flag: Exchanges old and new paths atomically.
const RENAME_EXCHANGE: c_int = 2;

// TODO implement flags
// TODO do not allow rename if the file is in use (example: cwd of a process, listing subfiles,
// etc...)

pub(super) fn do_renameat2(
	olddirfd: c_int,
	oldpath: SyscallString,
	newdirfd: c_int,
	newpath: SyscallString,
	_flags: c_int,
	fds: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// Get old file
	let oldpath = oldpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let old_parent_path = oldpath.parent().ok_or_else(|| errno!(ENOTDIR))?;
	let old_name = oldpath.file_name().ok_or_else(|| errno!(ENOENT))?;
	let old_parent = vfs::get_file_from_path(old_parent_path, &rs)?;
	let Resolved::Found(old) = at::get_file(&fds.lock(), rs.clone(), olddirfd, Some(&oldpath), 0)?
	else {
		return Err(errno!(ENOENT));
	};
	// Get new file
	let newpath = newpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	// TODO RENAME_NOREPLACE
	let Resolved::Creatable {
		parent: new_parent,
		name: new_name,
	} = at::get_file(&fds.lock(), rs.clone(), newdirfd, Some(&newpath), 0)?
	else {
		return Err(errno!(EEXIST));
	};
	// Create destination file
	{
		// If source and destination are on different mountpoints, error
		if new_parent.node().location.mountpoint_id != old.node().location.mountpoint_id {
			return Err(errno!(EXDEV));
		}
		// TODO Check permissions if sticky bit is set
		// Create link at new location
		// The `..` entry is already updated by the file system since having the same
		// directory in several locations is not allowed
		vfs::link(&new_parent, new_name, &old, &rs.access_profile)?;
	}
	// Remove source file
	// TODO on failure, undo previous creation
	vfs::unlink(old_parent, old_name, &rs.access_profile)?;
	Ok(0)
}

pub fn renameat2(
	Args((olddirfd, oldpath, newdirfd, newpath, flags)): Args<(
		c_int,
		SyscallString,
		c_int,
		SyscallString,
		c_int,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_renameat2(olddirfd, oldpath, newdirfd, newpath, flags, fds, rs)
}
