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
	file::{fd::FileDescriptorTable, socket::Socket},
	process::{mem_space::copy::SyscallSlice, Process},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{any::Any, ffi::c_int, intrinsics::unlikely};
use utils::{
	errno,
	errno::{EResult, Errno},
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
	if unlikely(addrlen < 0) {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let _sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Get slices
	let _buf_slice = buf.copy_from_user_vec(0, len)?.ok_or(errno!(EFAULT))?;
	let _dest_addr_slice = dest_addr
		.copy_from_user_vec(0, addrlen as usize)?
		.ok_or(errno!(EFAULT))?;
	todo!()
}
