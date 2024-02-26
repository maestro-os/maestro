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

//! This module defines sockaddr structures used by system calls to define connection informations
//! on sockets.

use super::Address;
use core::ffi::c_short;

/// Structure providing connection informations for sockets with IPv4.
#[repr(C)]
#[derive(Clone)]
pub struct SockAddrIn {
	/// The family of the socket.
	sin_family: c_short,
	/// The port on which the connection is to be opened.
	sin_port: c_short,
	/// The destination address of the connection.
	sin_addr: u32,
	/// Padding.
	sin_zero: [u8; 8],
}

/// Structure representing an IPv6 address.
#[repr(C)]
#[derive(Clone, Copy)]
pub union In6Addr {
	__s6_addr: [u8; 16],
	__s6_addr16: [u16; 8],
	__s6_addr32: [u32; 4],
}

/// Structure providing connection informations for sockets with IPv6.
#[repr(C)]
#[derive(Clone)]
pub struct SockAddrIn6 {
	/// The family of the socket.
	sin6_family: c_short,
	/// The port on which the connection is to be opened.
	sin6_port: c_short,
	/// TODO doc
	sin6_flowinfo: u32,
	/// The destination address of the connection.
	sin6_addr: In6Addr,
	/// TODO doc
	sin6_scope_id: u32,
}

/// A unified structure which contains data passed from userspace.
#[derive(Debug)]
pub struct SockAddr {
	/// The port used by the socket.
	pub port: u16,
	/// The destination address of the socket.
	pub addr: Address,
}

impl From<SockAddrIn> for SockAddr {
	fn from(val: SockAddrIn) -> Self {
		let addr: [u8; 4] = [
			((val.sin_addr >> 24) & 0xff) as u8,
			((val.sin_addr >> 16) & 0xff) as u8,
			((val.sin_addr >> 8) & 0xff) as u8,
			(val.sin_addr & 0xff) as u8,
		];

		Self {
			port: val.sin_port as _,
			addr: Address::IPv4(addr),
		}
	}
}

impl From<SockAddrIn6> for SockAddr {
	fn from(val: SockAddrIn6) -> Self {
		let addr = unsafe { val.sin6_addr.__s6_addr };

		Self {
			port: val.sin6_port as _,
			addr: Address::IPv6(addr),
		}
	}
}
