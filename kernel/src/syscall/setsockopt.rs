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

//! The `setsockopt` system call sets an option on a socket.

use crate::{
	file::{fd::FileDescriptorTable, socket::Socket},
	process::{mem_space::copy::UserSlice, Process},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{any::Any, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn setsockopt(
	Args((sockfd, level, optname, optval, optlen)): Args<(c_int, c_int, c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let optval = UserSlice::from_user(optval, optlen)?;
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Set opt
	let optval = optval.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	sock.set_opt(level, optname, &optval).map(|opt| opt as _)
}
