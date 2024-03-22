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

//! `netlink` is an interface between the kernel and userspace.

use core::mem::size_of;
use utils::{
	collections::{ring_buffer::RingBuffer, vec::Vec},
	errno::EResult,
	vec,
};

/// Netlink message header.
#[repr(C)]
struct NLMsgHdr {
	/// Length of message including header
	nlmsg_len: u32,
	/// Type of message content
	nlmsg_type: u16,
	/// Additional flags
	nlmsg_flags: u16,
	/// Sequence number
	nlmsg_seq: u32,
	/// Sender port ID
	nlmsg_pid: u32,
}

/// The netlink handle for a socket. Each socket must have its own instance.
#[derive(Debug)]
pub struct Handle {
	/// Socket family being used
	pub family: i32,

	/// The buffer for read operations.
	read_buff: RingBuffer<u8, Vec<u8>>,
	/// The buffer for write operations.
	write_buff: RingBuffer<u8, Vec<u8>>,
}

impl Handle {
	/// Creates a new handle.
	pub fn new() -> EResult<Self> {
		Ok(Self {
			family: 0,

			read_buff: RingBuffer::new(vec![0; 16384]?),
			write_buff: RingBuffer::new(vec![0; 16384]?),
		})
	}
}

impl Handle {
	/// Reads data from the I/O and writes it into `buff`.
	///
	/// The function returns the number of bytes read.
	pub fn read(&mut self, buff: &mut [u8]) -> EResult<(usize, bool)> {
		let len = self.read_buff.read(buff);
		Ok((len, false))
	}

	/// Reads data from `buff` and writes it into the I/O.
	///
	/// The function returns the number of bytes written.
	pub fn write(&mut self, buff: &[u8]) -> EResult<usize> {
		let len = self.write_buff.write(buff);

		// Read message header
		let mut buf: [u8; size_of::<NLMsgHdr>()] = [0; size_of::<NLMsgHdr>()];
		let l = self.write_buff.peek(buf.as_mut_slice());
		if l < buf.len() {
			return Ok(len);
		}
		let _hdr = buf.as_ptr() as *const NLMsgHdr;

		// TODO handle message
		todo!();
	}

	/// Tells whether the specified events are available on the I/O interface.
	///
	/// `mask` is a mask containing the mask of operations to check for.
	///
	/// The function returns the mask with available events set.
	pub fn poll(&mut self, _mask: u32) -> EResult<u32> {
		// TODO
		todo!();
	}
}
