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

//! The `shutdown` system call shuts down part of a full-duplex connection.

use crate::{
	file::{buffer, buffer::socket::Socket},
	process::Process,
};
use core::{any::Any, ffi::c_int};
use macros::syscall;
use utils::{errno, errno::Errno};

/// Shutdown receive side of the connection.
const SHUT_RD: c_int = 0;
/// Shutdown receive side of the connection.
const SHUT_WR: c_int = 1;
/// Both sides are shutdown.
const SHUT_RDWR: c_int = 2;

#[syscall]
pub fn shutdown(sockfd: c_int, how: c_int) -> Result<i32, Errno> {
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
	let sock_mutex = buffer::get(open_file.get_location()).ok_or_else(|| errno!(ENOENT))?;
	let mut sock = sock_mutex.lock();
	let sock = (&mut *sock as &mut dyn Any)
		.downcast_mut::<Socket>()
		.ok_or_else(|| errno!(ENOTSOCK))?;

	match how {
		SHUT_RD => sock.shutdown_receive(),
		SHUT_WR => sock.shutdown_transmit(),

		SHUT_RDWR => {
			sock.shutdown_receive();
			sock.shutdown_transmit();
		}

		_ => return Err(errno!(EINVAL)),
	}
	Ok(0)
}
