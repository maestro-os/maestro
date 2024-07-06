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

//! The `fchdir` system call allows to change the current working directory of the
//! current process.

use crate::{file::FileType, process::Process, syscall::Args};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
	TryClone,
};

pub fn fchdir(Args(fd): Args<c_int>) -> EResult<usize> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();
	let cwd = {
		// Get file
		let fds_mutex = proc.file_descriptors.as_ref().unwrap();
		let fds = fds_mutex.lock();
		let open_file_mutex = fds.get_fd(fd)?.get_open_file().clone();
		let open_file = open_file_mutex.lock();
		let file = open_file.get_file().lock();
		// Check the file is an accessible directory
		// Virtual files can only be FIFOs or sockets
		let Some(path) = open_file.get_path() else {
			return Err(errno!(ENOTDIR));
		};
		if file.stat.file_type != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !proc.access_profile.can_list_directory(&file) {
			return Err(errno!(EACCES));
		}
		(path.try_clone()?, open_file.get_file().clone())
	};
	proc.cwd = Arc::new(cwd)?;
	Ok(0)
}
