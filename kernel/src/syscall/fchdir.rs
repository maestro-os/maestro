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

use crate::{
	file::{fd::FileDescriptorTable, perm::AccessProfile, FileType},
	process::Process,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
	TryClone,
};

pub fn fchdir(
	Args(fd): Args<c_int>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
	proc: Arc<IntMutex<Process>>,
) -> EResult<usize> {
	let cwd = {
		// Get file
		let open_file_mutex = fds.lock().get_fd(fd)?.get_open_file().clone();
		let open_file = open_file_mutex.lock();
		let file_mutex = open_file.get_file().clone();
		let file = file_mutex.lock();
		// Check the file is an accessible directory
		// Virtual files can only be FIFOs or sockets
		let Some(path) = open_file.get_path() else {
			return Err(errno!(ENOTDIR));
		};
		if file.stat.file_type != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		if !ap.can_list_directory(&file) {
			return Err(errno!(EACCES));
		}
		drop(file);
		(path.try_clone()?, file_mutex)
	};
	proc.lock().cwd = Arc::new(cwd)?;
	Ok(0)
}
