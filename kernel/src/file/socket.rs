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
	file::{File, fs::FileOps, vfs},
	memory::{ring_buffer::RingBuffer, user::UserSlice},
	net::{SocketDesc, SocketDomain, osi, sockaddr::SockAddr},
	sync::{spin::Spin, wait_queue::WaitQueue},
	syscall::ioctl,
};
use core::{
	any::Any,
	ffi::{c_int, c_void},
	fmt::Debug,
	num::NonZeroUsize,
	sync::{atomic, atomic::AtomicUsize},
};
use utils::{
	boxed::Box,
	collections::vec::Vec,
	errno,
	errno::{AllocResult, EResult},
	ptr::arc::Arc,
};

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

/// Socket option level: Socket
const SOL_SOCKET: c_int = 1;

/// Socket opt: Send buffer size
const SO_SNDBUF: c_int = 7;
/// Socket opt: Receive buffer size
const SO_RCVBUF: c_int = 8;

/// Socket operations
pub trait SocketOps: Debug {
	/// Connects a socket as a client, with the given `addr`.
	fn connect(&self, sock: &Arc<Socket>, addr: SockAddr) -> EResult<()>;

	/// Read data from the socket.
	///
	/// `buf` is the buffer the data is written to.
	///
	/// On success, the function returns the number of bytes read.
	fn read(&self, sock: &Socket, buf: &mut [u8]) -> EResult<usize>;
	/// Writes data to the socket.
	///
	/// `buf` is the buffer the data is read from.
	///
	/// On success, the function returns the number of bytes written.
	fn write(&self, sock: &Socket, buf: &[u8]) -> EResult<usize>;
}

/// Unix socket operations
#[derive(Debug, Default)]
pub struct UnixSocketOps {
	/// If connected, contains the peer socket
	peer: Spin<Option<Arc<Socket>>>,
}

impl SocketOps for UnixSocketOps {
	fn connect(&self, sock: &Arc<Socket>, addr: SockAddr) -> EResult<()> {
		let SockAddr::Unix(sa) = addr else {
			return Err(errno!(EINVAL));
		};
		let file = vfs::get_file_from_path(sa.get_path(), true)?;
		// If not a socket file, error
		let dstsock = file.node().file_ops.as_ref();
		let dstsock: Option<&Socket> = (dstsock as &dyn Any).downcast_ref();
		let dstsock = dstsock.ok_or_else(|| errno!(ENOTSOCK))?;
		let mut p = self.peer.lock();
		if p.is_some() {
			return Err(errno!(EISCONN));
		}
		// Create peer socket
		let peer = Arc::new(Socket::new_with_ops(
			sock.desc.clone(),
			Box::new(UnixSocketOps {
				peer: Spin::new(Some(sock.clone())),
			})?,
		)?)?;
		let mut dst_backlog = dstsock.backlog.lock();
		// TODO if the backlog is full, wait? or return an error?
		dst_backlog.push(peer.clone())?;
		*p = Some(peer);
		Ok(())
	}

	fn read(&self, _sock: &Socket, _buf: &mut [u8]) -> EResult<usize> {
		todo!()
	}

	fn write(&self, _sock: &Socket, _buf: &[u8]) -> EResult<usize> {
		todo!()
	}
}

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

	/// The address the socket is bound to
	pub sockaddr: Spin<Option<SockAddr>>,
	/// Socket operations
	pub ops: Box<dyn SocketOps>,

	// TODO use a FIFO
	/// If this is a listening socket, this is a queue of pending connections to be accepted with
	/// the `accept` system call.
	///
	/// The size of the queue is defined by the `listen` system call.
	pub backlog: Spin<Vec<Arc<Socket>>>,

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
	/// Creates a new instance, with default socket operations
	pub fn new(desc: SocketDesc) -> AllocResult<Self> {
		let ops = match desc.domain {
			SocketDomain::AfUnix => Box::new(UnixSocketOps::default())?,
			SocketDomain::AfInet => todo!(),
			SocketDomain::AfInet6 => todo!(),
			SocketDomain::AfNetlink => todo!(),
			SocketDomain::AfPacket => todo!(),
		};
		Self::new_with_ops(desc, ops)
	}

	/// Creates a new instance, with the given socket operations `ops`
	pub fn new_with_ops(desc: SocketDesc, ops: Box<dyn SocketOps>) -> AllocResult<Self> {
		Ok(Self {
			desc,
			stack: None,
			open_count: AtomicUsize::new(0),

			sockaddr: Default::default(),
			ops,

			backlog: Default::default(),

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
	/// - `level` is the level (protocol) at which the option is located
	/// - `optname` is the name of the option
	/// - `optval` is the slice to write the value to
	///
	/// The function returns the length of the written value
	pub fn get_opt(&self, level: c_int, optname: c_int, optval: UserSlice<u8>) -> EResult<usize> {
		match level {
			SOL_SOCKET => match optname {
				SO_SNDBUF | SO_RCVBUF => {
					let val = BUFFER_SIZE as u32;
					let len = optval.copy_to_user(0, &val.to_ne_bytes())?;
					Ok(len)
				}
				_ => Err(errno!(EINVAL)),
			},
			_ => Err(errno!(EINVAL)),
		}
	}

	/// Writes the given socket option.
	///
	/// Arguments:
	/// - `level` is the level (protocol) at which the option is located
	/// - `optname` is the name of the option
	/// - `optval` is the value of the option
	///
	/// The function returns a value to be returned by the syscall on success.
	pub fn set_opt(&self, _level: c_int, _optname: c_int, _optval: &[u8]) -> EResult<c_int> {
		// TODO
		Ok(0)
	}

	/// Tells whether the socket is listening.
	pub fn is_listening(&self) -> bool {
		self.backlog.lock().capacity() > 0
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
