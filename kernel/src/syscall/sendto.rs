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

//! The `sendto` system call sends a message on a socket.

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
// TODO implement flags

#[allow(clippy::type_complexity)]
pub fn sendto(
	Args((sockfd, buf, len, _flags, dest_addr, addrlen)): Args<(
		c_int,
		SyscallSlice<u8>,
		usize,
		c_int,
		SyscallSlice<u8>,
		isize,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if addrlen < 0 {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let loc = *fds
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
	// Get slices
	let _buf_slice = buf.copy_from_user(..len)?.ok_or(errno!(EFAULT))?;
	let _dest_addr_slice = dest_addr
		.copy_from_user(..(addrlen as usize))?
		.ok_or(errno!(EFAULT))?;
	// TODO
	todo!()
}
