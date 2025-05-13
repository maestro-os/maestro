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

//! Socket interface system calls.

use crate::{
	file,
	file::{File, fd::FileDescriptorTable, perm::AccessProfile, socket::Socket},
	memory::user::{UserPtr, UserSlice},
	net::{SocketDesc, SocketDomain, SocketType},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{cmp::min, ffi::c_int, hint::unlikely};
use utils::{errno, errno::EResult, ptr::arc::Arc};

/// Shutdown receive side of the connection.
const SHUT_RD: c_int = 0;
/// Shutdown receive side of the connection.
const SHUT_WR: c_int = 1;
/// Both sides are shutdown.
const SHUT_RDWR: c_int = 2;

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
	let sock = Arc::new(Socket::new(desc)?)?;
	let file = File::open_floating(sock, file::O_RDWR)?;
	let (sock_fd_id, _) = fds.lock().create_fd(0, file)?;
	Ok(sock_fd_id as _)
}

pub fn socketpair(
	Args((domain, r#type, protocol, sv)): Args<(c_int, c_int, c_int, UserPtr<[c_int; 2]>)>,
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
	let sock = Arc::new(Socket::new(desc)?)?;
	let file0 = File::open_floating(sock.clone(), file::O_RDWR)?;
	let file1 = File::open_floating(sock, file::O_RDWR)?;
	// Create file descriptors
	let (fd0_id, fd1_id) = fds.lock().create_fd_pair(file0, file1)?;
	sv.copy_to_user(&[fd0_id as _, fd1_id as _])?;
	Ok(0)
}

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

pub fn getsockopt(
	Args((sockfd, level, optname, optval, optlen)): Args<(c_int, c_int, c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let val = sock.get_opt(level, optname)?;
	// Write
	let len = min(val.len(), optlen);
	let optval = UserSlice::from_user(optval, optlen)?;
	optval.copy_to_user(0, &val[..len])?;
	Ok(len as _)
}

pub fn setsockopt(
	Args((sockfd, level, optname, optval, optlen)): Args<(c_int, c_int, c_int, *mut u8, usize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let optval = UserSlice::from_user(optval, optlen)?;
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Set opt
	let optval = optval.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	sock.set_opt(level, optname, &optval).map(|opt| opt as _)
}

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

pub fn bind(
	Args((sockfd, addr, addrlen)): Args<(c_int, *mut u8, isize)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if addrlen < 0 {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let addr = UserSlice::from_user(addr, addrlen as _)?;
	let addr = addr.copy_from_user_vec(0)?.ok_or_else(|| errno!(EFAULT))?;
	sock.bind(&addr)?;
	Ok(0)
}

// TODO implement flags
#[allow(clippy::type_complexity)]
pub fn sendto(
	Args((sockfd, buf, len, _flags, dest_addr, addrlen)): Args<(
		c_int,
		*mut u8,
		usize,
		c_int,
		*mut u8,
		isize,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, len)?;
	let dest_addr = UserSlice::from_user(dest_addr, addrlen as _)?;
	// Validation
	if unlikely(addrlen < 0) {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let file = fds.lock().get_fd(sockfd)?.get_file().clone();
	let _sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Get slices
	let _buf_slice = buf.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	let _dest_addr_slice = dest_addr.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	todo!()
}

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
