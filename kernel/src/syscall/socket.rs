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

//! The `socket` system call allows to create a socket.

use crate::{
	file,
	file::{
		buffer,
		buffer::{socket::Socket, Buffer},
		fd::FileDescriptorTable,
		perm::AccessProfile,
		vfs, File,
	},
	net::{SocketDesc, SocketDomain, SocketType},
	process::Process,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	boxed::Box,
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn socket(
	Args((domain, r#type, protocol)): Args<(c_int, c_int, c_int)>,
	ap: AccessProfile,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let sock_domain = SocketDomain::try_from(domain as u32)?;
	let sock_type = SocketType::try_from(r#type as u32)?;
	// Check permissions
	if !ap.can_use_sock_domain(&sock_domain) || !ap.can_use_sock_type(&sock_type) {
		return Err(errno!(EACCES));
	}
	let desc = SocketDesc {
		domain: sock_domain,
		type_: sock_type,
		protocol,
	};
	// Create socket
	let sock = Buffer::new(Socket::new(desc)?)?;
	let file = File::open_ops(Box::new(sock)?, file::O_RDWR)?;
	let (sock_fd_id, _) = fds.lock().create_fd(0, file)?;
	Ok(sock_fd_id as _)
}
