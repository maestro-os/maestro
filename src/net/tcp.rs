//! The Transmission Control Protocol (TCP) is a protocol transmitting sequenced, reliable,
//! two-way, connection-based byte streams.

use crate::errno::Errno;
use crate::file::buffer::socket::Socket;
use super::BuffList;
use super::Layer;

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
	fn transmit<'c, F>(
		&self,
		_buff: BuffList<'c>,
		_next: F
	) -> Result<(), Errno>
		where F: Fn(BuffList<'c>) -> Result<(), Errno> {
		// TODO
		todo!();
	}
}

/// Initiates a TCP connection on the given socket `sock`.
pub fn init_connection(_sock: &mut Socket) -> Result<(), Errno> {
	// TODO
	todo!();
}
