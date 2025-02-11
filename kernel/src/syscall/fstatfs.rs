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

//! The `fstatfs` system call returns information about a mounted file system.

use crate::{
	file::{fd::FileDescriptorTable, fs::Statfs},
	process::{mem_space::copy::SyscallPtr, Process},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{ffi::c_int, intrinsics::size_of};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

/// Performs the `fstatfs` system call.
pub fn do_fstatfs(
	fd: c_int,
	_sz: usize,
	buf: SyscallPtr<Statfs>,
	fds: &FileDescriptorTable,
) -> EResult<usize> {
	// TODO use `sz`
	let stat = fds
		.get_fd(fd)?
		.get_file()
		.vfs_entry
		.as_ref()
		.ok_or_else(|| errno!(ENOSYS))?
		.node()
		.mp
		.fs
		.get_stat()?;
	buf.copy_to_user(&stat)?;
	Ok(0)
}

pub fn fstatfs(
	Args((fd, buf)): Args<(c_int, SyscallPtr<Statfs>)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	do_fstatfs(fd, size_of::<Statfs>(), buf, &fds.lock())
}
