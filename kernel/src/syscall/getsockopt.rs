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

//! The `getsockopt` system call gets an option on a socket.

use crate::{
	file::{fd::FileDescriptorTable, socket::Socket},
	memory::user::UserSlice,
	process::Process,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{any::Any, cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn getsockopt(
	Args((sockfd, level, optname, optval, optlen)): Args<(c_int, c_int, c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let val = sock.get_opt(level, optname)?;
	// Write
	let len = min(val.len(), optlen);
	let optval = UserSlice::from_user(optval, optlen)?;
	optval.copy_to_user(0, &val[..len])?;
	Ok(len as _)
}
