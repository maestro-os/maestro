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

//! The `socketpair` system call creates a pair of file descriptor to an unnamed
//! socket which can be used for IPC (Inter-Process Communication).

use crate::{
	file::{buffer, buffer::socket::Socket, open_file, open_file::OpenFile, vfs},
	net::{SocketDesc, SocketDomain, SocketType},
	process::{mem_space::ptr::SyscallPtr, Process},
};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn socketpair(
	domain: c_int,
	r#type: c_int,
	protocol: c_int,
	sv: SyscallPtr<[c_int; 2]>,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();
	let sv_slice = sv.get_mut(&mut mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let sock_domain = SocketDomain::try_from(domain as u32)?;
	let sock_type = SocketType::try_from(r#type as u32)?;
	if !proc.access_profile.can_use_sock_domain(&sock_domain)
		|| !proc.access_profile.can_use_sock_type(&sock_type)
	{
		return Err(errno!(EACCES));
	}
	let desc = SocketDesc {
		domain: sock_domain,
		type_: sock_type,
		protocol,
	};

	let sock = Socket::new(desc)?;
	let loc = buffer::register(None, sock)?;
	let file = vfs::get_file_from_location(&loc)?;

	let open_file0 = OpenFile::new(file.clone(), open_file::O_RDONLY)?;
	let open_file1 = OpenFile::new(file, open_file::O_WRONLY)?;

	let fds_mutex = proc.file_descriptors.as_ref().unwrap();
	let mut fds = fds_mutex.lock();
	let fd0 = fds.create_fd(0, open_file0)?;
	sv_slice[0] = fd0.get_id() as _;
	let fd1 = fds.create_fd(0, open_file1)?;
	sv_slice[1] = fd1.get_id() as _;

	Ok(0)
}
