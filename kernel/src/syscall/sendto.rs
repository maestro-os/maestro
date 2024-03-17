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
	file::{buffer, buffer::socket::Socket},
	process::{mem_space::ptr::SyscallSlice, Process},
};
use core::{any::Any, ffi::c_int};
use macros::syscall;
use utils::{errno, errno::Errno};

// TODO implement flags

#[syscall]
pub fn sendto(
	sockfd: c_int,
	buf: SyscallSlice<u8>,
	len: usize,
	_flags: c_int,
	dest_addr: SyscallSlice<u8>,
	addrlen: isize,
) -> Result<i32, Errno> {
	if sockfd < 0 {
		return Err(errno!(EBADF));
	}
	if addrlen < 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Get socket
	let fds_mutex = proc.file_descriptors.as_ref().unwrap();
	let fds = fds_mutex.lock();
	let fd = fds.get_fd(sockfd as _).ok_or_else(|| errno!(EBADF))?;
	let open_file_mutex = fd.get_open_file();
	let open_file = open_file_mutex.lock();
	let sock_mutex = buffer::get(open_file.get_location()).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let _sock = (&mut *sock as &mut dyn Any)
		.downcast_mut::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;

	// Get slices
	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let _buf_slice = buf.get(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;
	let _dest_addr_slice = dest_addr
		.get(&mem_space_guard, addrlen as _)?
		.ok_or(errno!(EFAULT))?;

	// TODO
	todo!()
}
