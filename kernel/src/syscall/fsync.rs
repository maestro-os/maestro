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

//! The `fsync` system call synchronizes the state of a file to storage.

use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn fsync(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let file_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.file_descriptors.as_ref().unwrap();
		let fds = fds_mutex.lock();

		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;

		let open_file_mutex = fd.get_open_file();
		let open_file = open_file_mutex.lock();

		open_file.get_file().clone()
	};

	let file = file_mutex.lock();
	file.sync()?;

	Ok(0)
}
