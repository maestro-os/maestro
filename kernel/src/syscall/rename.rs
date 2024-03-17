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

//! The `rename` system call renames a file.

use crate::{
	file::{path::PathBuf, vfs, vfs::ResolutionSettings, FileType},
	process::{mem_space::ptr::SyscallString, Process},
};
use macros::syscall;
use utils::{errno, errno::Errno};

// TODO implementation probably can be merged with `renameat2`
// TODO do not allow rename if the file is in use (example: cwd of a process, listing subfiles,
// etc...)

#[syscall]
pub fn rename(oldpath: SyscallString, newpath: SyscallString) -> Result<i32, Errno> {
	let (old_path, new_path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old_path = PathBuf::try_from(oldpath)?;

		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let new_path = PathBuf::try_from(newpath)?;

		let rs = ResolutionSettings::for_process(&proc, false);
		(old_path, new_path, rs)
	};

	let old_parent_path = old_path.parent().ok_or_else(|| errno!(ENOTDIR))?;
	let old_name = old_path.file_name().ok_or_else(|| errno!(ENOENT))?;

	let old_parent_mutex = vfs::get_file_from_path(old_parent_path, &rs)?;
	let mut old_parent = old_parent_mutex.lock();

	let old_mutex = vfs::get_file_from_path(&old_path, &rs)?;
	let mut old = old_mutex.lock();
	// Cannot rename mountpoint
	if old.is_mountpoint() {
		return Err(errno!(EBUSY));
	}

	let new_parent_path = new_path.parent().ok_or_else(|| errno!(ENOTDIR))?;
	let new_parent_mutex = vfs::get_file_from_path(
		new_parent_path,
		&ResolutionSettings {
			follow_link: true,
			..rs
		},
	)?;
	let new_parent = new_parent_mutex.lock();
	let new_name = new_path.file_name().ok_or_else(|| errno!(ENOENT))?;

	// If source and destination are on different mountpoints, error
	if new_parent.get_location().get_mountpoint_id() != old.get_location().get_mountpoint_id() {
		return Err(errno!(EXDEV));
	}

	// TODO Check permissions if sticky bit is set

	vfs::create_link(&new_parent, new_name, &mut old, &rs.access_profile)?;

	if old.get_type() != FileType::Directory {
		// TODO On fail, undo
		vfs::remove_file(&mut old_parent, old_name, &rs.access_profile)?;
	}

	Ok(0)
}
