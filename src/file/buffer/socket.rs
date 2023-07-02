//! This file implements sockets.

use super::Buffer;
use crate::errno::Errno;
use crate::file::buffer::BlockHandler;
use crate::net::osi;
use crate::net::SocketDesc;
use crate::net::SocketDomain;
use crate::net::SocketType;
use crate::process::mem_space::MemSpace;
use crate::process::Process;
use crate::syscall::ioctl;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;
use crate::util::io::IO;
use crate::util::lock::IntMutex;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryDefault;
use core::ffi::c_int;
use core::ffi::c_void;

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

	/// The buffer containing received data.
	receive_buffer: RingBuffer<u8, Vec<u8>>,
	/// The buffer containing sent data.
	send_buffer: RingBuffer<u8, Vec<u8>>,

	/// The number of entities owning a reference to the socket. When this count reaches zero, the
	/// socket is closed.
	open_count: u32,

	/// The socket's block handler.
	block_handler: BlockHandler,
}

impl Socket {
	/// Creates a new instance.
	pub fn new(desc: SocketDesc) -> Result<Arc<Mutex<Self>>, Errno> {
		Arc::new(Mutex::new(Self {
			desc,
			stack: None,

			receive_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),
			send_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),

			open_count: 0,

			block_handler: BlockHandler::new(),
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
	pub fn get_opt(
		&self,
		_level: c_int,
		_optname: c_int,
		_optval: &mut [u8],
	) -> Result<c_int, Errno> {
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
	pub fn set_opt(
		&mut self,
		_level: c_int,
		_optname: c_int,
		_optval: &[u8],
	) -> Result<c_int, Errno> {
		// TODO
		todo!()
	}
}

impl TryDefault for Socket {
	fn try_default() -> Result<Self, Errno> {
		let desc = SocketDesc {
			domain: SocketDomain::AfUnix,
			type_: SocketType::SockRaw,
			protocol: 0,
		};

		Ok(Self {
			desc,
			stack: None,

			receive_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),
			send_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),

			open_count: 0,

			block_handler: BlockHandler::new(),
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

	fn add_waiting_process(&mut self, proc: &mut Process, mask: u32) -> Result<(), Errno> {
		self.block_handler.add_waiting_process(proc, mask)
	}

	fn ioctl(
		&mut self,
		_mem_space: Arc<IntMutex<MemSpace>>,
		_request: ioctl::Request,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}

impl IO for Socket {
	fn get_size(&self) -> u64 {
		0
	}

	/// Note: This implemention ignores the offset.
	fn read(&mut self, _: u64, _buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		if !self.desc.type_.is_stream() {
			// TODO error
		}

		// TODO
		todo!();
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, _buf: &[u8]) -> Result<u64, Errno> {
		// A destination address is required
		let Some(_stack) = self.stack.as_ref() else {
			return Err(errno!(EDESTADDRREQ));
		};

		// TODO
		todo!();
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
