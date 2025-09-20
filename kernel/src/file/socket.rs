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

use crate::{
	file::{File, FileType, Stat, fs::FileOps, wait_queue::WaitQueue},
	memory::{ring_buffer::RingBuffer, user::UserSlice},
	net::{SocketDesc, osi},
	sync::spin::Spin,
	syscall::ioctl,
};
use core::{
	ffi::{c_int, c_void},
	num::NonZeroUsize,
	sync::{atomic, atomic::AtomicUsize},
};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{AllocResult, EResult},
};

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

/// Socket option level: Socket
const SOL_SOCKET: c_int = 1;

/// A UNIX socket.
#[derive(Debug)]
pub struct Socket {
	/// The socket's stack descriptor.
	desc: SocketDesc,
	/// The socket's network stack corresponding to the descriptor.
	stack: Option<osi::Stack>,
	/// The number of entities owning a reference to the socket. When this count reaches zero, the
	/// socket is closed.
	open_count: AtomicUsize,

	/// The address the socket is bound to.
	sockname: Spin<Vec<u8>>,

	/// The buffer containing received data. If `None`, reception has been shutdown.
	rx_buff: Spin<Option<RingBuffer>>,
	/// The buffer containing data to be transmitted. If `None`, transmission has been shutdown.
	tx_buff: Spin<Option<RingBuffer>>,

	/// Receive wait queue.
	rx_queue: WaitQueue,
	/// Transmit wait queue.
	tx_queue: WaitQueue,
}

impl Socket {
	/// Creates a new instance.
	pub fn new(desc: SocketDesc) -> AllocResult<Self> {
		Ok(Self {
			desc,
			stack: None,
			open_count: AtomicUsize::new(0),

			sockname: Default::default(),

			rx_buff: Spin::new(Some(RingBuffer::new(
				NonZeroUsize::new(BUFFER_SIZE).unwrap(),
			)?)),
			tx_buff: Spin::new(Some(RingBuffer::new(
				NonZeroUsize::new(BUFFER_SIZE).unwrap(),
			)?)),

			rx_queue: WaitQueue::new(),
			tx_queue: WaitQueue::new(),
		})
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
	pub fn get_opt(&self, _level: c_int, _optname: c_int) -> EResult<&[u8]> {
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
	pub fn set_opt(&self, _level: c_int, _optname: c_int, _optval: &[u8]) -> EResult<c_int> {
		// TODO
		Ok(0)
	}

	/// Returns the name of the socket.
	pub fn get_sockname(&self) -> &Spin<Vec<u8>> {
		&self.sockname
	}

	/// Binds the socket to the given address.
	///
	/// `sockaddr` is the new socket name.
	///
	/// If the socket is already bound, or if the address is invalid, or if the address is already
	/// in used, the function returns an error.
	pub fn bind(&self, sockaddr: &[u8]) -> EResult<()> {
		let mut sockname = self.sockname.lock();
		if !sockname.is_empty() {
			return Err(errno!(EINVAL));
		}
		// TODO check if address is already in used (EADDRINUSE)
		// TODO check the requested network interface exists (EADDRNOTAVAIL)
		// TODO check address against stack's domain

		*sockname = Vec::try_from(sockaddr)?;
		Ok(())
	}

	/// Shuts down the reception side of the socket.
	pub fn shutdown_reception(&self) {
		*self.rx_buff.lock() = None;
	}

	/// Shuts down the transmit side of the socket.
	pub fn shutdown_transmit(&self) {
		*self.tx_buff.lock() = None;
	}
}

impl FileOps for Socket {
	fn get_stat(&self, _file: &File) -> EResult<Stat> {
		Ok(Stat {
			mode: FileType::Socket.to_mode() | 0o666,
			..Default::default()
		})
	}

	fn acquire(&self, _file: &File) {
		self.open_count.fetch_add(1, atomic::Ordering::Acquire);
	}

	fn release(&self, _file: &File) {
		let cnt = self.open_count.fetch_sub(1, atomic::Ordering::Release);
		if cnt == 0 {
			// TODO close the socket
		}
	}

	fn poll(&self, _file: &File, _mask: u32) -> EResult<u32> {
		todo!()
	}

	fn ioctl(&self, _file: &File, _request: ioctl::Request, _argp: *const c_void) -> EResult<u32> {
		todo!()
	}

	fn read(&self, _file: &File, _off: u64, _buf: UserSlice<u8>) -> EResult<usize> {
		if !self.desc.type_.is_stream() {
			// TODO error
		}
		todo!()
	}

	fn write(&self, _file: &File, _off: u64, _buf: UserSlice<u8>) -> EResult<usize> {
		// A destination address is required
		let Some(_stack) = self.stack.as_ref() else {
			return Err(errno!(EDESTADDRREQ));
		};
		todo!()
	}
}
