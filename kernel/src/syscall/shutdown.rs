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
	file::{fd::FileDescriptorTable, socket::Socket},
	process::Process,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{any::Any, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

/// Shutdown receive side of the connection.
const SHUT_RD: c_int = 0;
/// Shutdown receive side of the connection.
const SHUT_WR: c_int = 1;
/// Both sides are shutdown.
const SHUT_RDWR: c_int = 2;

pub fn shutdown(
	Args((sockfd, how)): Args<(c_int, c_int)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Do shutdown
	match how {
		SHUT_RD => sock.shutdown_reception(),
		SHUT_WR => sock.shutdown_transmit(),
		SHUT_RDWR => {
			sock.shutdown_reception();
			sock.shutdown_transmit();
		}
		_ => return Err(errno!(EINVAL)),
	}
	Ok(0)
}
