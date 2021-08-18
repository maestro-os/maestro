//! This file implements sockets.

use crate::errno::Errno;
use crate::util::container::vec::Vec;
use crate::util::ptr::SharedPtr;

// TODO Figure out the behaviour when opening socket file more than twice at a time

/// Structure representing a socket.
pub struct Socket {
	/// The socket's domain.
	domain: i32,
	/// The socket's type.
	type_: i32,
	/// The socket's protocol.
	protocol: i32,

	// TODO Handle network sockets

	/// The list of sides of the socket.
	sides: Vec<SharedPtr<SocketSide>>,
}

impl Socket {
	/// Creates a new instance.
	pub fn new(domain: i32, type_: i32, protocol: i32) -> Result<SharedPtr<Self>, Errno> {
		// TODO Check domain, type and protocol. Use EINVAL, EPROTOTYPE and EPROTONOSUPPORT

		SharedPtr::new(Self {
			domain,
			type_,
			protocol,

			sides: Vec::new(),
		})
	}
}

/// A side of a socket is a structure which allows to read/write from the socket. It is required to
/// prevent one side from reading the data it wrote itself.
pub struct SocketSide {
	/// The socket.
	sock: SharedPtr<Socket>,
}

impl SocketSide {
	/// Creates a new instance.
	pub fn new(sock: SharedPtr<Socket>) -> Self {
		Self {
			sock,
		}
	}

	/// Reads data from the socket.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, _buf: &mut [u8]) -> usize {
		// TODO
		todo!();
	}

	/// Writes data to the socket.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, _buf: &[u8]) -> usize {
		// TODO
		todo!();
	}
}
