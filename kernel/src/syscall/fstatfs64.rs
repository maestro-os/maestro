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

//! The `fstatfs64` system call returns information about a mounted file system.

use crate::{
	file::fs::Statfs,
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn fstatfs64(
	Args((fd, _sz, buf)): Args<(c_int, usize, SyscallPtr<Statfs>)>,
) -> EResult<usize> {
	// TODO use `sz`

	let file_mutex = {
		let proc_mutex = Process::current();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.file_descriptors.as_ref().unwrap();
		let fds = fds_mutex.lock();

		let fd = fds.get_fd(fd)?;

		let open_file_mutex = fd.get_open_file();
		let open_file = open_file_mutex.lock();

		open_file.get_file().clone()
	};

	let file = file_mutex.lock();

	let mountpoint_mutex = file.location.get_mountpoint().unwrap();
	let mountpoint = mountpoint_mutex.lock();

	let fs = mountpoint.get_filesystem();
	let stat = fs.get_stat()?;

	// Write the statfs structure to userspace
	buf.copy_to_user(stat)?;

	Ok(0)
}
