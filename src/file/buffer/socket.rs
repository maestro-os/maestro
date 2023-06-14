//! This file implements sockets.

use super::Buffer;
use crate::errno::Errno;
use crate::file::buffer::BlockHandler;
use crate::net;
use crate::net::ip;
use crate::net::ip::IPv4Layer;
use crate::net::osi::Layer;
use crate::net::tcp::TCPLayer;
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
#[derive(Debug)]
pub struct Socket {
	/// The socket's stack descriptor.
	desc: SocketDesc,

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
		Arc::new(Mutex::new(Self {
			desc,

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
}

impl TryDefault for Socket {
	fn try_default() -> Result<Self, Errno> {
		Ok(Self {
			desc: SocketDesc {
				domain: SocketDomain::AfUnix,
				type_: SocketType::SockRaw,
				protocol: 0,
			},

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
		// TODO
		todo!();
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, buf: &[u8]) -> Result<u64, Errno> {
		match &mut self.desc.domain {
			SocketDomain::AfUnix => todo!(), // TODO

			dom @ (SocketDomain::AfInet | SocketDomain::AfInet6) => {
				let transport = match self.desc.type_ {
					SocketType::SockStream => TCPLayer {},
					SocketType::SockDgram => todo!(),     // TODO
					SocketType::SockSeqpacket => todo!(), // TODO
					SocketType::SockRaw => todo!(),       // TODO
				};

				let network = match dom {
					SocketDomain::AfInet => IPv4Layer {
						protocol: match self.desc.type_ {
							SocketType::SockStream => ip::PROTO_TCP,
							SocketType::SockDgram => ip::PROTO_UDP,
							SocketType::SockSeqpacket => todo!(), // TODO
							SocketType::SockRaw => todo!(),       // TODO
						},

						dst_addr: [0; 4], // TODO
					},

					SocketDomain::AfInet6 => todo!(), // TODO

					_ => unreachable!(),
				};

				// TODO use real dst addr
				if let Some(iface) = net::get_iface_for(net::Address::IPv4([0; 4])) {
					network.transmit(buf.into(), |buf| {
						transport.transmit(buf, |buf| {
							let mut iface = iface.lock();
							// TODO retry if not everything has been written
							iface.write(&buf)?;

							Ok(())
						})
					})?;

					Ok(buf.len() as _)
				} else {
					// TODO error (errno to be determined)
					todo!();
				}
			}

			SocketDomain::AfNetlink(n) => {
				n.family = self.desc.protocol;

				let len = n.write(buf)?;
				Ok(len as u64)
			}

			SocketDomain::AfPacket => {
				match self.desc.type_ {
					SocketType::SockDgram => todo!(), // TODO

					SocketType::SockRaw => {
						if let Some(iface) = net::get_iface_for(net::Address::IPv4([0; 4])) {
							let mut iface = iface.lock();
							iface.write(&buf.into())?;

							Ok(buf.len() as _)
						} else {
							// TODO error (errno to be determined)
							todo!();
						}
					}

					_ => todo!(), // TODO invalid
				}
			}
		}
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
