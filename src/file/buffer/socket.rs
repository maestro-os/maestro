//! This file implements sockets.

use crate::net::osi;
use super::Buffer;
use crate::errno::Errno;
use crate::file::buffer::BlockHandler;
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
use core::ffi::c_void;

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

/// Structure representing a socket.
pub struct Socket {
	/// The socket's stack descriptor.
	desc: SocketDesc,
	/// The socket's network stack corresponding to the descriptor.
	stack: osi::Stack,

	/// The buffer containing received data.
	receive_buffer: RingBuffer<u8, Vec<u8>>,
	/// The buffer containing sent data.
	send_buffer: RingBuffer<u8, Vec<u8>>,

	/// The socket's block handler.
	block_handler: BlockHandler,
}

impl Socket {
	/// Creates a new instance.
	pub fn new(desc: SocketDesc) -> Result<Arc<Mutex<Self>>, Errno> {
		let stack = osi::get_stack(&desc).ok_or_else(|| errno!(EINVAL))?;

		Arc::new(Mutex::new(Self {
			desc,
			stack,

			receive_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),
			send_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),

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
	pub fn stack(&self) -> &osi::Stack {
		&self.stack
	}
}

impl TryDefault for Socket {
	fn try_default() -> Result<Self, Errno> {
		let desc = SocketDesc {
			domain: SocketDomain::AfUnix,
			type_: SocketType::SockRaw,
			protocol: 0,
		};
		let stack = osi::get_stack(&desc).unwrap();

		Ok(Self {
			desc,
			stack,

			receive_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),
			send_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),

			block_handler: BlockHandler::new(),
		})
	}
}

impl Buffer for Socket {
	fn get_capacity(&self) -> usize {
		// TODO
		todo!();
	}

	fn increment_open(&mut self, _read: bool, _write: bool) {
		// TODO
		todo!();
	}

	fn decrement_open(&mut self, _read: bool, _write: bool) {
		// TODO
		todo!();
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
		if !self.desc.is_stream() {
			// TODO error
		}

		// TODO
		todo!();
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, _buf: &[u8]) -> Result<u64, Errno> {
		if !self.desc.is_stream() {
			// TODO error only if no address has been set using `connect`
			return Err(errno!(EDESTADDRREQ));
		}

		// TODO
		todo!();
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
