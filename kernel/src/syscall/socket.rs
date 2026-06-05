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
	file::{File, FileType, O_RDWR, fd::fd_to_file, fs::float, socket::Socket},
	memory::user::{UserPtr, UserSlice},
	net::{
		SocketDesc, SocketDomain, SocketType,
		sockaddr::{SockAddr, SockAddrIn, SockAddrIn6, SockAddrUn},
	},
	process::Process,
	syscall::FromSyscallArg,
};
use core::{ffi::c_int, hint::unlikely};
use utils::{bytes, errno, errno::EResult};

/// Socket [`accept4`] flag: sets `O_NONBLOCK` on the newly open socket
const SOCK_NONBLOCK: c_int = 0o4000;
/// Socket [`accept4`] flag: sets `O_CLOEXEC` on the newly open socket
const SOCK_CLOEXEC: c_int = 0o2000000;

/// Shutdown receive side of the connection.
const SHUT_RD: c_int = 0;
/// Shutdown receive side of the connection.
const SHUT_WR: c_int = 1;
/// Both sides are shutdown.
const SHUT_RDWR: c_int = 2;

pub fn socket(domain: c_int, r#type: c_int, protocol: c_int) -> EResult<usize> {
	let sock_domain = SocketDomain::try_from(domain as u32)?;
	let sock_type = SocketType::try_from(r#type as u32)?;
	// Check permissions
	if unlikely(!sock_domain.can_use() || !sock_type.can_use()) {
		return Err(errno!(EACCES));
	}
	let desc = SocketDesc {
		domain: sock_domain,
		type_: sock_type,
		protocol,
	};
	// Create socket
	let sock = float::get_entry(Socket::new(desc)?, FileType::Socket)?;
	let file = File::open_floating(sock, O_RDWR)?;
	let (sock_fd_id, _) = Process::current()
		.file_descriptors()
		.lock()
		.create_fd(0, file)?;
	Ok(sock_fd_id as _)
}

pub fn socketpair(
	domain: c_int,
	r#type: c_int,
	protocol: c_int,
	sv: UserPtr<[c_int; 2]>,
) -> EResult<usize> {
	let sock_domain = SocketDomain::try_from(domain as u32)?;
	let sock_type = SocketType::try_from(r#type as u32)?;
	// Check permissions
	if unlikely(!sock_domain.can_use() || !sock_type.can_use()) {
		return Err(errno!(EACCES));
	}
	let desc = SocketDesc {
		domain: sock_domain,
		type_: sock_type,
		protocol,
	};
	// Create socket
	let sock = float::get_entry(Socket::new(desc)?, FileType::Socket)?;
	let file0 = File::open_floating(sock.clone(), O_RDWR)?;
	let file1 = File::open_floating(sock, O_RDWR)?;
	// Create file descriptors
	let (fd0_id, fd1_id) = Process::current()
		.file_descriptors()
		.lock()
		.create_fd_pair(file0, file1)?;
	sv.copy_to_user(&[fd0_id as _, fd1_id as _])?;
	Ok(0)
}

pub fn getsockname(sockfd: c_int, addr: *mut u8, addrlen: UserPtr<isize>) -> EResult<usize> {
	// Get socket
	let file = fd_to_file(sockfd)?;
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Read and check buffer length
	let addrlen_val = addrlen.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	if unlikely(addrlen_val < 0) {
		return Err(errno!(EINVAL));
	}
	let sockaddr = sock.sockaddr.lock();
	let len = if let Some(sockaddr) = &*sockaddr {
		let bytes = match sockaddr {
			SockAddr::Unix(sa) => bytes::as_bytes(sa),
			SockAddr::Inet(sa) => bytes::as_bytes(sa),
			SockAddr::Inet6(sa) => bytes::as_bytes(sa),
			SockAddr::Link(sa) => bytes::as_bytes(sa),
		};
		let addr = UserSlice::from_user(addr, addrlen_val as usize)?;
		addr.copy_to_user(0, bytes)?
	} else {
		0
	};
	addrlen.copy_to_user(&(len as _))?;
	Ok(0)
}

pub fn getsockopt(
	sockfd: c_int,
	level: c_int,
	optname: c_int,
	optval: *mut u8,
	optlen: UserPtr<usize>,
) -> EResult<usize> {
	let file = fd_to_file(sockfd)?;
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let len = optlen.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let optval = UserSlice::from_user(optval, len)?;
	let len = sock.get_opt(level, optname, optval)?;
	optlen.copy_to_user(&len)?;
	Ok(0)
}

pub fn setsockopt(
	sockfd: c_int,
	level: c_int,
	optname: c_int,
	optval: *mut u8,
	optlen: usize,
) -> EResult<usize> {
	let optval = UserSlice::from_user(optval, optlen)?;
	// Get socket
	let file = fd_to_file(sockfd)?;
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Set opt
	let optval = optval.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	sock.set_opt(level, optname, &optval).map(|opt| opt as _)
}

pub fn connect(sockfd: c_int, addr: *mut u8, addrlen: isize) -> EResult<usize> {
	// Validation
	if unlikely(addrlen < 0) {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let file = fd_to_file(sockfd)?;
	let _sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let addr = UserSlice::from_user(addr, addrlen as _)?;
	let _addr = addr.copy_from_user_vec(0)?.ok_or_else(|| errno!(EFAULT))?;
	// TODO connect socket
	todo!()
}

pub fn bind(sockfd: c_int, sockaddr: *const u8, addrlen: isize) -> EResult<usize> {
	// Get socket
	let file = fd_to_file(sockfd)?;
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	let dom = sock.desc().domain;
	// Check sockaddr length
	if unlikely(addrlen < 0 || addrlen as usize != dom.get_sockaddr_len()) {
		return Err(errno!(EINVAL));
	}
	// Note: the `compat` argument of `UserPtr` is ignored
	let sockaddr = match dom {
		SocketDomain::AfUnix => {
			let sockaddr = UserPtr::<SockAddrUn>::from_syscall_arg(sockaddr as _, false);
			let sockaddr = sockaddr.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
			if unlikely(sockaddr.sun_family != dom.get_id()) {
				return Err(errno!(EINVAL));
			}
			SockAddr::Unix(sockaddr)
		}
		SocketDomain::AfInet => {
			let sockaddr = UserPtr::<SockAddrIn>::from_syscall_arg(sockaddr as _, false);
			let sockaddr = sockaddr.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
			if unlikely(sockaddr.sin_family != dom.get_id()) {
				return Err(errno!(EINVAL));
			}
			SockAddr::Inet(sockaddr)
		}
		SocketDomain::AfInet6 => {
			let sockaddr = UserPtr::<SockAddrIn6>::from_syscall_arg(sockaddr as _, false);
			let sockaddr = sockaddr.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
			if unlikely(sockaddr.sin6_family != dom.get_id()) {
				return Err(errno!(EINVAL));
			}
			SockAddr::Inet6(sockaddr)
		}
		SocketDomain::AfNetlink => todo!(),
		SocketDomain::AfPacket => todo!(),
	};
	// TODO check if address is already in used (EADDRINUSE)
	// TODO check the requested network interface exists (EADDRNOTAVAIL)
	let mut sa = sock.sockaddr.lock();
	// If already bound, error
	if unlikely(sa.is_some()) {
		return Err(errno!(EINVAL));
	}
	*sa = Some(sockaddr);
	Ok(0)
}

pub fn listen(sockfd: c_int, backlog: c_int) -> EResult<usize> {
	if unlikely(backlog <= 0) {
		return Err(errno!(EINVAL));
	}
	let file = fd_to_file(sockfd)?;
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	match sock.desc().domain {
		SocketDomain::AfInet | SocketDomain::AfInet6 => {
			// If not bound to an address, error
			if unlikely(sock.sockaddr.lock().is_none()) {
				// TODO verify this is the correct errno
				return Err(errno!(EOPNOTSUPP));
			}
			// TODO if another socket is already listening on the same port, EADDRINUSE
		}
		SocketDomain::AfNetlink => return Err(errno!(EOPNOTSUPP)),
		_ => {}
	}
	*sock.backlog.lock() = backlog as _;
	Ok(0)
}

pub fn accept(sockfd: c_int, addr: *mut u8, addrlen: UserPtr<usize>) -> EResult<usize> {
	accept4(sockfd, addr, addrlen, 0)
}

pub fn accept4(
	sockfd: c_int,
	_addr: *mut u8,
	_addrlen: UserPtr<usize>,
	_flags: c_int,
) -> EResult<usize> {
	let file = fd_to_file(sockfd)?;
	let sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// If not a connection-based stream, error
	if unlikely(!sock.desc().type_.is_stream()) {
		return Err(errno!(EOPNOTSUPP));
	}
	// If the socket is not listening, error
	if unlikely(sock.is_listening()) {
		return Err(errno!(EINVAL));
	}
	// TODO if there is a pending connection:
	// - create a socket (use `flags`)
	// - write `addr` and `addrlen`
	// - return the socket's fd
	// else, wait for a new connection
	todo!()
}

// TODO implement flags
pub fn sendto(
	sockfd: c_int,
	buf: *mut u8,
	len: usize,
	_flags: c_int,
	dest_addr: *mut u8,
	addrlen: isize,
) -> EResult<usize> {
	let buf = UserSlice::from_user(buf, len)?;
	let dest_addr = UserSlice::from_user(dest_addr, addrlen as _)?;
	// Validation
	if unlikely(addrlen < 0) {
		return Err(errno!(EINVAL));
	}
	// Get socket
	let file = fd_to_file(sockfd)?;
	let _sock: &Socket = file.get_buffer().ok_or_else(|| errno!(ENOTSOCK))?;
	// Get slices
	let _buf_slice = buf.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	let _dest_addr_slice = dest_addr.copy_from_user_vec(0)?.ok_or(errno!(EFAULT))?;
	todo!()
}

pub fn shutdown(sockfd: c_int, how: c_int) -> EResult<usize> {
	// Get socket
	let file = fd_to_file(sockfd)?;
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
