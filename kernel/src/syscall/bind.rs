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

//! The `bind` system call binds a name to a socket.

use crate::{
	file::{buffer, buffer::socket::Socket, fd::FileDescriptorTable},
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use core::{any::Any, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn bind(
	Args((sockfd, addr, addrlen)): Args<(c_int, SyscallSlice<u8>, isize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if addrlen < 0 {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let loc = fds
		.lock()
		.get_fd(sockfd)?
		.get_file()
		.lock()
		.get_location()
		.clone();
	let sock = buffer::get(&loc).ok_or_else(|| errno!(ENOENT))?;
	let sock = (&*sock as &dyn Any)
		.downcast_ref::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;
	let addr = addr
		.copy_from_user(..(addrlen as usize))?
		.ok_or_else(|| errno!(EFAULT))?;
	sock.bind(&addr)?;
	Ok(0)
}
