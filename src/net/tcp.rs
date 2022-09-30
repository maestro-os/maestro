//! The Transmission Control Protocol (TCP) is a protocol transmitting sequenced, reliable,
//! two-way, connection-based byte streams.

use crate::errno::Errno;
use crate::file::socket::Socket;

/// Initiates a TCP connection on the given socket `sock`.
pub fn init_connection(_sock: &mut Socket) -> Result<(), Errno> {
	// TODO
	todo!();
}
