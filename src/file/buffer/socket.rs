//! This file implements sockets.

use super::Buffer;
use crate::errno::Errno;
use crate::file::buffer::BlockHandler;
use crate::net;
use crate::net::ip;
use crate::net::ip::IPv4Layer;
use crate::net::osi::Layer;
use crate::net::sockaddr::SockAddr;
use crate::net::sockaddr::SockAddrIn;
use crate::net::sockaddr::SockAddrIn6;
use crate::net::tcp;
use crate::net::tcp::TCPLayer;
use crate::net::SocketDesc;
use crate::net::SocketDomain;
use crate::net::SocketType;
use crate::process::mem_space::MemSpace;
use crate::process::Process;
use crate::syscall::ioctl;
use crate::util;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;
use crate::util::io::IO;
use crate::util::lock::IntMutex;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryDefault;
use core::ffi::c_short;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

/// Enumeration of socket states.
#[derive(Clone, Copy, Debug)]
pub enum SockState {
	/// The socket has just been created.
	Created,
	/// The socket is waiting for acknowledgement after issuing a connection.
	WaitingAck,
	/// The socket is ready for I/O.
	Ready,
	// TODO Closed state?
}

/// Structure representing a socket.
#[derive(Debug)]
pub struct Socket {
	/// The socket's stack descriptor.
	desc: SocketDesc,

	/// The state of the socket.
	state: SockState,
	/// Informations about the socket's destination.
	sockaddr: Option<SockAddr>,

	// TODO Handle network sockets
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
		// TODO Check domain, type and protocol

		Arc::new(Mutex::new(Self {
			desc,

			state: SockState::Created,
			sockaddr: None,

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

	/// Returns the current state of the socket.
	#[inline(always)]
	pub fn get_state(&self) -> SockState {
		self.state
	}

	/// Connects the socket with the address specified in the structure represented by `sockaddr`.
	///
	/// If the structure is invalid or if the connection cannot succeed, the function returns an
	/// error.
	///
	/// If the function succeeds, the caller must wait until the state of the socket turns to
	/// `Ready`.
	pub fn connect(&mut self, sockaddr: &[u8]) -> Result<(), Errno> {
		// Check whether the slice is large enough to hold the structure type
		if sockaddr.len() < size_of::<c_short>() {
			return Err(errno!(EINVAL));
		}

		// Getting the family
		let mut sin_family: c_short = 0;
		unsafe {
			ptr::copy_nonoverlapping::<c_short>(
				&sockaddr[0] as *const _ as *const _,
				&mut sin_family,
				1,
			);
		}

		let domain = SocketDomain::try_from(sin_family as u32)?;
		if sockaddr.len() < domain.get_sockaddr_len() {
			return Err(errno!(EINVAL));
		}

		let sockaddr: SockAddr = match domain {
			SocketDomain::AfInet => unsafe { util::reinterpret::<SockAddrIn>(sockaddr) }
				.unwrap()
				.clone()
				.into(),

			SocketDomain::AfInet6 => unsafe { util::reinterpret::<SockAddrIn6>(sockaddr) }
				.unwrap()
				.clone()
				.into(),

			_ => return Err(errno!(EPROTOTYPE)),
		};

		self.sockaddr = Some(sockaddr);

		// Opening connection if necessary
		match self.desc.type_ {
			SocketType::SockStream => {
				tcp::init_connection(self)?;
				self.state = SockState::WaitingAck;
			}

			SocketType::SockSeqpacket => {
				// TODO
				todo!();
			}

			_ => self.state = SockState::Ready,
		}

		Ok(())
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

			state: SockState::Created,
			sockaddr: None,

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
					network.transmit(buf.into(), |bufs| {
						transport.transmit(bufs, |bufs| {
							let buff = bufs.collect()?;
							let mut iface = iface.lock();
							iface.write(&buff)?;

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
							iface.write(buf)?;

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
