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

use crate::{
	file::{path::PathBuf, vfs, vfs::ResolutionSettings, FileType},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn chdir(Args(path): Args<SyscallString>) -> EResult<usize> {
	let (path, rs) = {
		let proc_mutex = Process::current();
		let proc = proc_mutex.lock();

		let path = path.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let rs = ResolutionSettings::for_process(&proc, true);
		(path, rs)
	};
	let dir_mutex = vfs::get_file_from_path(&path, &rs)?;
	// Validation
	{
		let dir = dir_mutex.lock();
		if dir.stat.file_type != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !rs.access_profile.can_list_directory(&dir) {
			return Err(errno!(EACCES));
		}
	};
	// Set new cwd
	{
		let proc_mutex = Process::current();
		let mut proc = proc_mutex.lock();
		proc.cwd = Arc::new((path, dir_mutex))?;
	}
	Ok(0)
}
