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

//! The `getsockname` system call returns the socket address bound to a socket.

use crate::{
	file::{buffer, buffer::socket::Socket},
	process::{
		mem_space::ptr::{SyscallPtr, SyscallSlice},
		Process,
	},
};
use core::{any::Any, ffi::c_int};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn getsockname(
	sockfd: c_int,
	addr: SyscallSlice<u8>,
	addrlen: SyscallPtr<isize>,
) -> Result<i32, Errno> {
	if sockfd < 0 {
		return Err(errno!(EBADF));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Get socket
	let fds_mutex = proc.file_descriptors.as_ref().unwrap();
	let fds = fds_mutex.lock();
	let fd = fds.get_fd(sockfd as _).ok_or_else(|| errno!(EBADF))?;
	let open_file_mutex = fd.get_open_file();
	let open_file = open_file_mutex.lock();
	let loc = open_file.get_location();
	let sock_mutex = buffer::get(loc).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let sock = (&mut *sock as &mut dyn Any)
		.downcast_mut::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	// Read and check buffer length
	let addrlen_val = addrlen
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;
	if *addrlen_val < 0 {
		return Err(errno!(EINVAL));
	}
	let addrlen_val = *addrlen_val as usize;

	// Read socket name
	let addr_slice = addr
		.get_mut(&mut mem_space_guard, addrlen_val)?
		.ok_or(errno!(EFAULT))?;
	let len = sock.read_sockname(addr_slice) as _;

	// Update actual length of the address
	let addrlen_val = addrlen
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;
	*addrlen_val = len;

	Ok(0)
}
