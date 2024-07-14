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

//! The `connect` system call connects a socket to a distant host.

use crate::{
	file::{buffer, buffer::socket::Socket, fd::FileDescriptorTable},
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use core::{any::Any, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
};

/// The implementation of the `connect` syscall.
pub fn connect(
	Args((sockfd, addr, addrlen)): Args<(c_int, SyscallSlice<u8>, isize)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if addrlen < 0 {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let loc = *fds_mutex
		.lock()
		.get_fd(sockfd)?
		.get_open_file()
		.lock()
		.get_location();
	let sock_mutex = buffer::get(&loc).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let _sock = (&mut *sock as &mut dyn Any)
		.downcast_mut::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;
	let _addr = addr
		.copy_from_user(addrlen as _)?
		.ok_or_else(|| errno!(EFAULT))?;
	// TODO connect socket
	todo!();
}
