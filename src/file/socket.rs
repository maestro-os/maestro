//! This file implements sockets.

use crate::errno::Errno;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;
use crate::util::ptr::SharedPtr;

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

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

	/// The buffer containing received data.
	receive_buffer: RingBuffer,
	/// The buffer containing sent data.
	send_buffer: RingBuffer,

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

			receive_buffer: RingBuffer::new(BUFFER_SIZE)?,
			send_buffer: RingBuffer::new(BUFFER_SIZE)?,

			sides: Vec::new(),
		})
	}

	/// Returns the socket's domain.
	#[inline(always)]
	pub fn get_domain(&self) -> i32 {
		self.domain
	}

	/// Returns the socket's type.
	#[inline(always)]
	pub fn get_type(&self) -> i32 {
		self.type_
	}

	/// Returns the socket's protocol.
	#[inline(always)]
	pub fn get_protocol(&self) -> i32 {
		self.protocol
	}
}

/// A side of a socket is a structure which allows to read/write from the socket. It is required to
/// prevent one side from reading the data it wrote itself.
pub struct SocketSide {
	/// The socket.
	sock: SharedPtr<Socket>,

	/// Tells which side is the current side.
	other: bool,
}

impl SocketSide {
	/// Creates a new instance.
	/// `sock` is the socket associated with the socket side.
	/// `other` allows to tell on which side is which.
	pub fn new(sock: SharedPtr<Socket>, other: bool) -> Result<SharedPtr<Self>, Errno> {
		let s = SharedPtr::new(Self {
			sock: sock.clone(),
			other,
		});

		{
			let mut guard = sock.lock();
			guard.get_mut().sides.push(s.clone()?)?;
		}

		s
	}

	/// Reads data from the socket.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, buf: &mut [u8]) -> usize {
		let mut guard = self.sock.lock();
		let sock = guard.get_mut();

		if self.other {
			sock.send_buffer.read(buf)
		} else {
			sock.receive_buffer.read(buf)
		}
	}

	/// Writes data to the socket.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, buf: &[u8]) -> usize {
		let mut guard = self.sock.lock();
		let sock = guard.get_mut();

		if self.other {
			sock.receive_buffer.write(buf)
		} else {
			sock.send_buffer.write(buf)
		}
	}
}
