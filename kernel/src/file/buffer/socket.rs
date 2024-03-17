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

//! This file implements sockets.

use super::Buffer;
use crate::{
	file::buffer::BlockHandler,
	net::{osi, SocketDesc, SocketDomain, SocketType},
	process::{mem_space::MemSpace, Process},
	syscall::ioctl,
};
use core::{
	cmp::min,
	ffi::{c_int, c_void},
};
use utils::{
	collections::{ring_buffer::RingBuffer, vec::Vec},
	errno,
	errno::{AllocResult, EResult},
	io::IO,
	lock::{IntMutex, Mutex},
	ptr::arc::Arc,
	vec, TryDefault,
};

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

/// Socket option level: Socket
const SOL_SOCKET: c_int = 1;

/// Structure representing a socket.
pub struct Socket {
	/// The socket's stack descriptor.
	desc: SocketDesc,
	/// The socket's network stack corresponding to the descriptor.
	stack: Option<osi::Stack>,

	/// The buffer containing received data. If `None`, reception has been shutdown.
	receive_buffer: Option<RingBuffer<u8, Vec<u8>>>,
	/// The buffer containing data to be transmitted. If `None`, transmission has been shutdown.
	transmit_buffer: Option<RingBuffer<u8, Vec<u8>>>,

	/// The number of entities owning a reference to the socket. When this count reaches zero, the
	/// socket is closed.
	open_count: u32,

	/// The socket's block handler.
	block_handler: BlockHandler,

	/// The address the socket is bound to.
	sockname: Vec<u8>,
}

impl Socket {
	/// Creates a new instance.
	pub fn new(desc: SocketDesc) -> AllocResult<Arc<Mutex<Self>>> {
		Arc::new(Mutex::new(Self {
			desc,
			stack: None,

			receive_buffer: Some(RingBuffer::new(vec![0; BUFFER_SIZE]?)),
			transmit_buffer: Some(RingBuffer::new(vec![0; BUFFER_SIZE]?)),

			open_count: 0,

			block_handler: BlockHandler::new(),

			sockname: Vec::new(),
		}))
	}

	/// Returns the socket's descriptor.
	#[inline(always)]
	pub fn desc(&self) -> &SocketDesc {
		&self.desc
	}

	/// Returns the socket's network stack.
	#[inline(always)]
	pub fn stack(&self) -> Option<&osi::Stack> {
		self.stack.as_ref()
	}

	/// Reads the given socket option.
	///
	/// Arguments:
	/// - `level` is the level (protocol) at which the option is located.
	/// - `optname` is the name of the option.
	/// - `optval` is the value of the option.
	///
	/// The function returns a value to be returned by the syscall on success.
	pub fn get_opt(&self, _level: c_int, _optname: c_int, _optval: &mut [u8]) -> EResult<c_int> {
		// TODO
		todo!()
	}

	/// Writes the given socket option.
	///
	/// Arguments:
	/// - `level` is the level (protocol) at which the option is located.
	/// - `optname` is the name of the option.
	/// - `optval` is the value of the option.
	///
	/// The function returns a value to be returned by the syscall on success.
	pub fn set_opt(&mut self, _level: c_int, _optname: c_int, _optval: &[u8]) -> EResult<c_int> {
		// TODO
		Ok(0)
	}

	/// Writes the bound socket name into `sockaddr`.
	/// If the buffer is too small, the address is truncated.
	///
	/// The function returns the length of the socket address.
	pub fn read_sockname(&self, sockaddr: &mut [u8]) -> usize {
		let len = min(sockaddr.len(), self.sockname.len());
		sockaddr[..len].copy_from_slice(&self.sockname);

		self.sockname.len()
	}

	/// Tells whether the socket is bound.
	pub fn is_bound(&self) -> bool {
		!self.sockname.is_empty()
	}

	/// Binds the socket to the given address.
	///
	/// `sockaddr` is the new socket name.
	///
	/// If the socket is already bound, or if the address is invalid, or if the address is already
	/// in used, the function returns an error.
	pub fn bind(&mut self, sockaddr: &[u8]) -> EResult<()> {
		if self.is_bound() {
			return Err(errno!(EINVAL));
		}
		// TODO check if address is already in used (EADDRINUSE)
		// TODO check the requested network interface exists (EADDRNOTAVAIL)
		// TODO check address against stack's domain

		self.sockname = Vec::from_slice(sockaddr)?;
		Ok(())
	}

	/// Shuts down the receive side of the socket.
	pub fn shutdown_receive(&mut self) {
		self.receive_buffer = None;
	}

	/// Shuts down the transmit side of the socket.
	pub fn shutdown_transmit(&mut self) {
		self.transmit_buffer = None;
	}
}

impl TryDefault for Socket {
	fn try_default() -> Result<Self, Self::Error> {
		let desc = SocketDesc {
			domain: SocketDomain::AfUnix,
			type_: SocketType::SockRaw,
			protocol: 0,
		};

		Ok(Self {
			desc,
			stack: None,

			receive_buffer: Some(RingBuffer::new(vec![0; BUFFER_SIZE]?)),
			transmit_buffer: Some(RingBuffer::new(vec![0; BUFFER_SIZE]?)),

			open_count: 0,

			block_handler: BlockHandler::new(),

			sockname: Default::default(),
		})
	}
}

impl Buffer for Socket {
	fn get_capacity(&self) -> usize {
		// TODO
		todo!()
	}

	fn increment_open(&mut self, _read: bool, _write: bool) {
		self.open_count += 1;
	}

	fn decrement_open(&mut self, _read: bool, _write: bool) {
		self.open_count -= 1;
		if self.open_count == 0 {
			// TODO close the socket
		}
	}

	fn add_waiting_process(&mut self, proc: &mut Process, mask: u32) -> EResult<()> {
		self.block_handler.add_waiting_process(proc, mask)
	}

	fn ioctl(
		&mut self,
		_mem_space: Arc<IntMutex<MemSpace>>,
		_request: ioctl::Request,
		_argp: *const c_void,
	) -> EResult<u32> {
		// TODO
		todo!();
	}
}

impl IO for Socket {
	fn get_size(&self) -> u64 {
		0
	}

	/// Note: This implemention ignores the offset.
	fn read(&mut self, _: u64, _buf: &mut [u8]) -> EResult<(u64, bool)> {
		if !self.desc.type_.is_stream() {
			// TODO error
		}

		// TODO
		todo!();
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, _buf: &[u8]) -> EResult<u64> {
		// A destination address is required
		let Some(_stack) = self.stack.as_ref() else {
			return Err(errno!(EDESTADDRREQ));
		};

		// TODO
		todo!();
	}

	fn poll(&mut self, _mask: u32) -> EResult<u32> {
		// TODO
		todo!();
	}
}
