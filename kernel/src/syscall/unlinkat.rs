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

//! The `unlinkat` syscall allows to unlink a file.
//!
//! If no link remain to the file, the function also removes it.

use super::util::at;
use crate::{
	file::{
		path::PathBuf,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{mem_space::ptr::SyscallString, Process},
	syscall::util::at::AT_EMPTY_PATH,
};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn unlinkat(dirfd: c_int, pathname: SyscallString, flags: c_int) -> Result<i32, Errno> {
	let (fds_mutex, path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, false);

		let mem_space = proc.get_mem_space().unwrap().clone();
		let mem_space_guard = mem_space.lock();

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		let pathname = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(pathname)?;

		(fds_mutex, path, rs)
	};

	let fds = fds_mutex.lock();
	let parent_path = path.parent().ok_or_else(|| errno!(ENOENT))?;
	// AT_EMPTY_PATH is required in case the path has only one component
	let resolved = at::get_file(&fds, rs.clone(), dirfd, parent_path, flags | AT_EMPTY_PATH)?;
	match resolved {
		Resolved::Found(parent_mutex) => {
			let mut parent = parent_mutex.lock();
			let name = path.file_name().ok_or_else(|| errno!(ENOENT))?;
			vfs::remove_file(&mut parent, name, &rs.access_profile)?;
		}
		_ => return Err(errno!(ENOENT)),
	}

	Ok(0)
}
