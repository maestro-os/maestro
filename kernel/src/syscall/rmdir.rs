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

//! The `rmdir` system call a link to the given directory from its filesystem.
//!
//! If no link remain to the directory, the function also removes it.

use crate::{
	file::{path::PathBuf, vfs, vfs::ResolutionSettings},
	process::{mem_space::ptr::SyscallString, Process},
};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn rmdir(pathname: SyscallString) -> Result<i32, Errno> {
	let (path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, true);

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		(path, rs)
	};

	// Remove the directory
	{
		// Get directory
		let file_mutex = vfs::get_file_from_path(&path, &rs)?;
		let file = file_mutex.lock();
		// Validation
		match file.get_content() {
			// The 2 entries in question are `.` and `..`
			FileContent::Directory(entries) if entries.len() > 2 => return Err(errno!(ENOTEMPTY)),
			FileContent::Directory(_) => {}
			_ => return Err(errno!(ENOTDIR)),
		}
		// Remove
		vfs::remove_file_from_path(&path, &rs)?;
	}

	Ok(0)
}
