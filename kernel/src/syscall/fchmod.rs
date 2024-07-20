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

//! The `fchmod` system call allows change the permissions on a file.

use crate::{
	file,
	file::{fd::FileDescriptorTable, perm::AccessProfile},
	process::Process,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn fchmod(
	Args((fd, mode)): Args<(c_int, file::Mode)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	ap: AccessProfile,
) -> EResult<usize> {
	let file_mutex = fds_mutex
		.lock()
		.get_fd(fd)?
		.get_open_file()
		.lock()
		.get_file()
		.clone();
	let mut file = file_mutex.lock();
	// Check permissions
	if !ap.can_set_file_permissions(&file) {
		return Err(errno!(EPERM));
	}
	// Update
	file.stat.set_permissions(mode as _);
	// TODO lazy sync
	file.sync()?;
	Ok(0)
}
