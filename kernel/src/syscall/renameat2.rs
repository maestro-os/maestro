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
		path::PathBuf,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		FileType,
	},
	process::{mem_space::ptr::SyscallString, Process},
};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno};

/// Flag: Don't replace new path if it exists. Return an error instead.
const RENAME_NOREPLACE: c_int = 1;
/// Flag: Exchanges old and new paths atomically.
const RENAME_EXCHANGE: c_int = 2;

// TODO implement flags
// TODO do not allow rename if the file is in use (example: cwd of a process, listing subfiles,
// etc...)

#[syscall]
pub fn renameat2(
	olddirfd: c_int,
	oldpath: SyscallString,
	newdirfd: c_int,
	newpath: SyscallString,
	_flags: c_int,
) -> Result<i32, Errno> {
	let (fds_mutex, oldpath, newpath, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, false);

		let mem_space = proc.get_mem_space().unwrap().clone();
		let mem_space_guard = mem_space.lock();

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let oldpath = PathBuf::try_from(oldpath)?;

		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let newpath = PathBuf::try_from(newpath)?;

		(fds_mutex, oldpath, newpath, rs)
	};

	let fds = fds_mutex.lock();

	let old_parent_path = oldpath.parent().ok_or_else(|| errno!(ENOTDIR))?;
	let old_name = oldpath.file_name().ok_or_else(|| errno!(ENOENT))?;

	let old_parent_mutex = vfs::get_file_from_path(old_parent_path, &rs)?;
	let mut old_parent = old_parent_mutex.lock();

	let Resolved::Found(old_mutex) = at::get_file(&fds, rs.clone(), olddirfd, &oldpath, 0)? else {
		return Err(errno!(ENOENT));
	};
	let mut old = old_mutex.lock();
	// Cannot rename mountpoint
	if old.is_mountpoint() {
		return Err(errno!(EBUSY));
	}

	// TODO RENAME_NOREPLACE
	let Resolved::Creatable {
		parent: new_parent,
		name: new_name,
	} = at::get_file(&fds, rs.clone(), newdirfd, &newpath, 0)?
	else {
		return Err(errno!(EEXIST));
	};
	let new_parent = new_parent.lock();

	// If source and destination are on different mountpoints, error
	if new_parent.get_location().get_mountpoint_id() != old.get_location().get_mountpoint_id() {
		return Err(errno!(EXDEV));
	}

	// TODO Check permissions if sticky bit is set

	// TODO On fail, undo

	// Create link at new location
	// The `..` entry is already updated by the file system since having the same
	// directory in several locations is not allowed
	vfs::create_link(&new_parent, new_name, &mut old, &rs.access_profile)?;

	if old.get_type() != FileType::Directory {
		vfs::remove_file(&mut old_parent, old_name, &rs.access_profile)?;
	}

	Ok(0)
}
