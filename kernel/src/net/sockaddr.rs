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

//! `sockaddr` structures used by system calls to define a socket's address.

use core::{
	ffi::{c_short, c_uchar, c_uint, c_ushort},
	fmt::{Debug, Formatter},
};
use macros::AnyRepr;
use utils::collections::path::Path;

/// POSIX's `sa_family`
pub type SaFamily = u16;

/// Unix domain socket address
#[repr(C)]
#[derive(AnyRepr, Clone, Copy, Debug)]
pub struct SockAddrUn {
	/// Socket family
	pub sun_family: SaFamily,
	/// Unix socket path
	pub sun_path: [u8; 108],
}

impl SockAddrUn {
	/// Returns the socket's path
	pub fn get_path(&self) -> &Path {
		let path_len = self
			.sun_path
			.iter()
			.position(|b| *b == 0)
			.unwrap_or(self.sun_path.len());
		// `sun_path`'s length is smaller than the maximum path length, so we can use
		// `new_unbounded`
		Path::new_unbounded(&self.sun_path[..path_len])
	}
}

/// IPv4 socket address
#[repr(C)]
#[derive(AnyRepr, Clone, Copy, Debug)]
pub struct SockAddrIn {
	/// Socket family
	pub sin_family: SaFamily,
	/// Port
	pub sin_port: c_short,
	/// Address
	pub sin_addr: u32,
	/// Padding
	pub sin_zero: [u8; 8],
}

/// An IPv6 address
#[repr(C)]
#[derive(AnyRepr, Clone, Copy)]
#[allow(missing_docs)]
pub union In6Addr {
	pub __s6_addr: [u8; 16],
	pub __s6_addr16: [u16; 8],
	pub __s6_addr32: [u32; 4],
}

impl Debug for In6Addr {
	fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
		unsafe {
			write!(
				f,
				"{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}",
				self.__s6_addr16[0],
				self.__s6_addr16[1],
				self.__s6_addr16[2],
				self.__s6_addr16[3],
				self.__s6_addr16[4],
				self.__s6_addr16[5],
				self.__s6_addr16[6],
				self.__s6_addr16[7]
			)
		}
	}
}

/// IPv6 socket address
#[repr(C)]
#[derive(AnyRepr, Clone, Copy, Debug)]
pub struct SockAddrIn6 {
	/// Socket family
	pub sin6_family: SaFamily,
	/// Port
	pub sin6_port: c_short,
	/// TODO doc
	pub sin6_flowinfo: u32,
	/// Address
	pub sin6_addr: In6Addr,
	/// TODO doc
	pub sin6_scope_id: u32,
}

/// Link-layer socket address
#[repr(C)]
#[derive(AnyRepr, Clone, Copy, Debug)]
pub struct SockAddrLl {
	/// Socket family
	pub sll_family: SaFamily,
	/// TODO doc
	pub sll_protocol: c_ushort,
	/// Interface index
	pub sll_ifindex: c_uint,
	/// TODO doc
	pub sll_hatype: c_ushort,
	/// TODO doc
	pub sll_pkttype: c_uchar,
	/// TODO doc
	pub sll_halen: c_uchar,
	/// TODO doc
	pub sll_addr: [c_uchar; 8],
}

/// Socket address
#[derive(Debug)]
pub enum SockAddr {
	/// Unix domain socket address
	Unix(SockAddrUn),
	/// IPv4 socket address
	Inet(SockAddrIn),
	/// IPv6 socket address
	Inet6(SockAddrIn6),
	/// Link-layer socket address
	Link(SockAddrLl),
}
