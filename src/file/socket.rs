//! This file implements sockets.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::process::mem_space::MemSpace;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;
use crate::util::io::IO;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

// TODO Figure out the behaviour when opening socket file more than twice at a time

/// Structure representing a socket.
#[derive(Debug)]
pub struct Socket {
	/// The socket's domain.
	domain: i32,
	/// The socket's type.
	type_: i32,
	/// The socket's protocol.
	protocol: i32,

	// TODO Handle network sockets
	/// The buffer containing received data.
	receive_buffer: RingBuffer<u8, Vec<u8>>,
	/// The buffer containing sent data.
	send_buffer: RingBuffer<u8, Vec<u8>>,

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

			receive_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),
			send_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),

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
#[derive(Debug)]
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
			let guard = sock.lock();
			guard.get_mut().sides.push(s.clone()?)?;
		}

		s
	}

	/// Performs an ioctl operation on the socket.
	pub fn ioctl(
		&mut self,
		_mem_space: IntSharedPtr<MemSpace>,
		_request: u32,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}

impl IO for SocketSide {
	fn get_size(&self) -> u64 {
		// TODO
		0
	}

	/// Note: This implemention ignores the offset.
	fn read(&mut self, _: u64, buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		let guard = self.sock.lock();
		let sock = guard.get_mut();

		if self.other {
			Ok((sock.send_buffer.read(buf) as _, false)) // TODO Handle EOF
		} else {
			Ok((sock.receive_buffer.read(buf) as _, false)) // TODO Handle EOF
		}
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, buf: &[u8]) -> Result<u64, Errno> {
		let guard = self.sock.lock();
		let sock = guard.get_mut();

		if self.other {
			Ok(sock.receive_buffer.write(buf) as _)
		} else {
			Ok(sock.send_buffer.write(buf) as _)
		}
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
