/*
 * Copyright 2026 Luc Lenôtre
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

//! Files polling.

use crate::{
	file::{File, fs::FileOps},
	sync::mutex::Mutex,
};
use core::ffi::c_long;
use utils::{collections::hashmap::HashMap, errno::EResult, ptr::arc::Arc};

/// The number of file descriptors in FDSet.
pub const FD_SETSIZE: usize = 1024;

/// Structure representing `fd_set`.
#[repr(C)]
#[derive(Debug)]
pub struct FDSet {
	/// The set's bitfield.
	fds_bits: [c_long; FD_SETSIZE / c_long::BITS as usize],
}

impl FDSet {
	/// Tells whether the given file descriptor `fd` is set in the list.
	pub fn is_set(&self, fd: u32) -> bool {
		if fd as usize >= FD_SETSIZE {
			return false;
		}
		// TODO Check correctness
		let i = (fd as usize) / c_long::BITS as usize;
		self.fds_bits[i] >> (fd % c_long::BITS) != 0
	}

	/// Sets or clears the bit for file descriptor `fd`.
	pub fn set(&mut self, fd: u32, val: bool) {
		// TODO Check correctness
		let i = (fd as usize) / c_long::BITS as usize;
		if val {
			self.fds_bits[i] |= 1 << (fd % c_long::BITS);
		} else {
			self.fds_bits[i] &= !(1 << (fd % c_long::BITS));
		}
	}
}

/// Poll event: There is data to read.
pub const POLLIN: u32 = 0x1;
/// Poll event: There is some exceptional condition on the file descriptor.
pub const POLLPRI: u32 = 0x2;
/// Poll event: Writing is now possible.
pub const POLLOUT: u32 = 0x4;
/// Poll event: Error condition.
pub const POLLERR: u32 = 0x8;
/// Poll event: Hang up.
pub const POLLHUP: u32 = 0x10;
/// Poll event: Invalid request.
pub const POLLNVAL: u32 = 0x20;
/// Poll event: Equivalent to POLLIN.
pub const POLLRDNORM: u32 = 0x40;
/// Poll event: Priority band data can be read.
pub const POLLRDBAND: u32 = 0x80;
/// Poll event: Equivalent to POLLOUT.
pub const POLLWRNORM: u32 = 0x100;
/// Poll event: Priority data may be written.
pub const POLLWRBAND: u32 = 0x200;
/// Poll event: Stream socket peer closed connection, or shut down writing half
/// of connection.
pub const POLLRDHUP: u32 = 0x2000;

/// A file descriptor passed to the `poll` system call.
#[repr(C)]
#[derive(Debug)]
pub struct PollFD {
	/// The file descriptor.
	pub fd: i32,
	/// The input mask telling which events to look for.
	pub events: i16,
	/// The output mask telling which events happened.
	pub revents: i16,
}

/// An epoll event, matching the userspace `struct epoll_event`.
///
/// **Note**: on x86 this structure is *packed*, hence the `repr(C, packed)`.
#[repr(C, packed)]
#[derive(Clone, Copy, Debug)]
pub struct EpollEvent {
	/// The events bitmask.
	pub events: u32,
	/// Opaque user data associated with the entry.
	pub data: u64,
}

/// An entry of the interest list: a file being watched by an epoll instance.
#[derive(Debug)]
pub struct EpollItem {
	/// The watched open file description.
	pub file: Arc<File>,
	/// The events the user is interested in (poll bits only, without the
	/// behaviour flags).
	pub events: u32,
	/// The behaviour flags ([`EPOLLET`] and [`EPOLLONESHOT`]).
	pub flags: u32,
	/// Opaque user data associated with the entry.
	pub data: u64,
	/// For edge-triggered entries, the set of events last reported to userspace.
	///
	/// An event is only reported again once it has been observed as not ready in
	/// between (rising edge).
	pub reported: u32,
}

/// The interest list of an epoll instance.
#[derive(Debug, Default)]
pub struct EpollFileOps(pub Mutex<HashMap<*const File, EpollItem>, false>);

impl FileOps for EpollFileOps {
	fn poll(&self, _file: &File, _mask: u32) -> EResult<u32> {
		// Nested epoll is not implemented
		Ok(0)
	}
}
