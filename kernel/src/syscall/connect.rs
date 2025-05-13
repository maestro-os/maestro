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
	file::{fd::FileDescriptorTable, socket::Socket},
	memory::user::UserSlice,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{ffi::c_int, intrinsics::unlikely};
use utils::{errno, errno::EResult, ptr::arc::Arc};

/// The implementation of the `connect` syscall.
pub fn connect(
	Args((sockfd, addr, addrlen)): Args<(c_int, *mut u8, isize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if unlikely(addrlen < 0) {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let _sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let addr = UserSlice::from_user(addr, addrlen as _)?;
	let _addr = addr.copy_from_user_vec(0)?.ok_or_else(|| errno!(EFAULT))?;
	// TODO connect socket
	todo!()
}
