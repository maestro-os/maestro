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

//! The fchdir system call allows to change the current working directory of the
//! current process.

use crate::{file::FileType, process::Process};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno, ptr::arc::Arc};

#[syscall]
pub fn fchdir(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let (open_file_mutex, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.file_descriptors.as_ref().unwrap();
		let fds = fds_mutex.lock();

		let open_file_mutex = fds
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()
			.clone();

		(open_file_mutex, proc.access_profile)
	};
	let open_file = open_file_mutex.lock();

	let (path, location) = {
		let file = open_file.get_file().lock();

		// Check for errors
		if file.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !ap.can_list_directory(&file) {
			return Err(errno!(EACCES));
		}

		(file.get_path()?.to_path_buf()?, file.get_location().clone())
	};

	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();
	proc.cwd = Arc::new((path, location))?;

	Ok(0)
}
