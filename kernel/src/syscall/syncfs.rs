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

//! The `syncfs` system call allows to synchronize the filesystem containing the
//! file pointed by the given file descriptor.

use crate::{file::fd::FileDescriptorTable, process::Process, syscall::Args};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn syncfs(Args(fd): Args<c_int>, fds: Arc<Mutex<FileDescriptorTable>>) -> EResult<usize> {
	let _mountpoint = fds
		.lock()
		.get_fd(fd)?
		.get_file()
		.lock()
		.vfs_entry
		.location
		.get_mountpoint();
	// TODO Sync all files on mountpoint
	Ok(0)
}
