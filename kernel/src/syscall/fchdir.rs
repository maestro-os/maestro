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
	sync::mutex::Mutex,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
	TryClone,
};

pub fn fchdir(
	Args(fd): Args<c_int>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
	proc: Arc<Process>,
) -> EResult<usize> {
	let file = fds
		.lock()
		.get_fd(fd)?
		.get_file()
		.vfs_entry
		.clone()
		.ok_or_else(|| errno!(ENOTDIR))?;
	let stat = file.stat()?;
	// Check the file is an accessible directory
	if stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !ap.can_list_directory(&stat) {
		return Err(errno!(EACCES));
	}
	proc.fs.lock().cwd = file;
	Ok(0)
}
