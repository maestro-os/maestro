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
	file::{fd::FileDescriptorTable, socket::Socket},
	process::{
		mem_space::copy::{UserPtr, UserSlice},
		Process,
	},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{any::Any, cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn getsockname(
	Args((sockfd, addr, addrlen)): Args<(c_int, *mut u8, UserPtr<isize>)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Read and check buffer length
	let addrlen_val = addrlen.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	if addrlen_val < 0 {
		return Err(errno!(EINVAL));
	}
	let name = sock.get_sockname().lock();
	let len = min(name.len(), addrlen_val as _);
	let addr = UserSlice::from_user(addr, len)?;
	addr.copy_to_user(0, &name[..len])?;
	addrlen.copy_to_user(&(len as _))?;
	Ok(0)
}
