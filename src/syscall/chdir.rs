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

//! The chdir system call allows to change the current working directory of the
//! current process.

use crate::errno;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::ptr::arc::Arc;
use macros::syscall;

#[syscall]
pub fn chdir(path: SyscallString) -> Result<i32, Errno> {
	let (new_cwd, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path_str = path.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let new_cwd = super::util::get_absolute_path(&proc, Path::from_str(path_str, true)?)?;

		(new_cwd, proc.access_profile)
	};

	{
		let dir_mutex = vfs::get_file_from_path(&new_cwd, &ap, true)?;
		let dir = dir_mutex.lock();

		// Check for errors
		if dir.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !ap.can_list_directory(&dir) {
			return Err(errno!(EACCES));
		}
	}

	// Set new cwd
	{
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();
		proc.cwd = Arc::new(new_cwd)?;
	}

	Ok(0)
}
