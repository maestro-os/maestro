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

//! The Transmission Control Protocol (TCP) is a protocol transmitting sequenced, reliable,
//! two-way, connection-based byte streams.

use super::{buff::BuffList, osi::Layer};
use crate::file::buffer::socket::Socket;
use utils::errno::EResult;

/// The TCP segment header.
#[repr(C, packed)]
pub struct TCPHdr {
	/// Source port.
	src_port: u16,
	/// Destination port.
	dst_port: u16,

	/// Sequence number.
	seq_nbr: u32,

	/// TODO doc
	ack_nbr: u32,

	/// The size of the header in units of 4 bytes.
	///
	/// Since the first 4 bits are reserved, the value must be shifted by 4 bits.
	data_offset: u8,
	/// The segment's flags.
	flags: u8,
	/// TODO doc
	win_size: u16,

	/// TODO doc
	checksum: u16,
	/// TODO doc
	urg_ptr: u16,
}

/// The network layer for the TCP protocol.
pub struct TCPLayer {}

impl Layer for TCPLayer {
	fn transmit<'c, F>(&self, _buff: BuffList<'c>, _next: F) -> EResult<()>
	where
		F: Fn(BuffList<'c>) -> EResult<()>,
	{
		// TODO
		todo!();
	}
}

/// Initiates a TCP connection on the given socket `sock`.
pub fn init_connection(_sock: &mut Socket) -> EResult<()> {
	// TODO
	todo!();
}
