//! This file implements sockets.

use core::ffi::c_short;
use core::ffi::c_void;
use core::mem::size_of;
use core::ptr;
use crate::errno::Errno;
use crate::file::Gid;
use crate::file::ROOT_GID;
use crate::file::ROOT_UID;
use crate::file::Uid;
use crate::file::buffer::BlockHandler;
use crate::net::BuffList;
use crate::net::Layer;
use crate::net::ip::IPv4Layer;
use crate::net::ip;
use crate::net::sockaddr::SockAddr;
use crate::net::sockaddr::SockAddrIn6;
use crate::net::sockaddr::SockAddrIn;
use crate::net::tcp::TCPLayer;
use crate::net::tcp;
use crate::net;
use crate::process::Process;
use crate::process::mem_space::MemSpace;
use crate::syscall::ioctl;
use crate::util::FailableDefault;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;
use crate::util::io::IO;
use crate::util::ptr::IntSharedPtr;
use crate::util::ptr::SharedPtr;
use crate::util;
use super::Buffer;

/// The maximum size of a socket's buffers.
const BUFFER_SIZE: usize = 65536;

/// Enumeration of socket domains.
#[derive(Copy, Clone, Debug)]
pub enum SockDomain {
	/// Local communication.
	AfUnix,
	/// IPv4 Internet Protocols.
	AfInet,
	/// IPv6 Internet Protocols.
	AfInet6,
	/// Kernel user interface device.
	AfNetlink,
	/// Low level packet interface.
	AfPacket,
}

impl SockDomain {
	/// Returns the domain associated with the given id.
	///
	/// If the id doesn't match any, the function returns `None`.
	pub fn from(id: i32) -> Option<Self> {
		match id {
			1 => Some(Self::AfUnix),
			2 => Some(Self::AfInet),
			10 => Some(Self::AfInet6),
			16 => Some(Self::AfNetlink),
			17 => Some(Self::AfPacket),

			_ => None,
		}
	}

	/// Tells whether the given user has the permission to use the socket domain.
	pub fn can_use(&self, uid: Uid, gid: Gid) -> bool {
		match self {
			Self::AfPacket => uid == ROOT_UID || gid == ROOT_GID,
			_ => true,
		}
	}

	/// Returns the size of the sockaddr structure for the domain.
	pub fn get_sockaddr_len(&self) -> usize {
		match self {
			Self::AfInet => size_of::<SockAddrIn>(),
			Self::AfInet6 => size_of::<SockAddrIn6>(),

			_ => 0,
		}
	}
}

/// Enumeration of socket types.
#[derive(Copy, Clone, Debug)]
pub enum SockType {
	/// Sequenced, reliable, two-way, connection-based byte streams.
	SockStream,
	/// Datagrams.
	SockDgram,
	/// Sequenced, reliable, two-way connection-based data transmission path for datagrams of fixed
	/// maximum length.
	SockSeqpacket,
	/// Raw network protocol access.
	SockRaw,
}

impl SockType {
	/// Returns the type associated with the given id.
	///
	/// If the id doesn't match any, the function returns `None`.
	pub fn from(id: i32) -> Option<Self> {
		match id {
			1 => Some(Self::SockStream),
			2 => Some(Self::SockDgram),
			5 => Some(Self::SockSeqpacket),
			3 => Some(Self::SockRaw),

			_ => None,
		}
	}

	/// Tells whether the given user has the permission to use the socket type.
	pub fn can_use(&self, uid: Uid, gid: Gid) -> bool {
		match self {
			Self::SockRaw => uid == ROOT_UID || gid == ROOT_GID,
			_ => true,
		}
	}
}

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
	/// The socket's domain.
	domain: SockDomain,
	/// The socket's type.
	type_: SockType,
	/// The socket's protocol.
	protocol: i32,

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
	pub fn new(domain: SockDomain, type_: SockType, protocol: i32)
		-> Result<SharedPtr<Self>, Errno> {
		// TODO Check domain, type and protocol

		SharedPtr::new(Self {
			domain,
			type_,
			protocol,

			state: SockState::Created,
			sockaddr: None,

			receive_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),
			send_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),

			block_handler: BlockHandler::new(),
		})
	}

	/// Returns the socket's domain.
	#[inline(always)]
	pub fn get_domain(&self) -> SockDomain {
		self.domain
	}

	/// Returns the socket's type.
	#[inline(always)]
	pub fn get_type(&self) -> SockType {
		self.type_
	}

	/// Returns the socket's protocol.
	#[inline(always)]
	pub fn get_protocol(&self) -> i32 {
		self.protocol
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
				1
			);
		}

		let domain = SockDomain::from(sin_family as _).ok_or_else(|| errno!(EAFNOSUPPORT))?;
		if sockaddr.len() < domain.get_sockaddr_len() {
			return Err(errno!(EINVAL));
		}

		let sockaddr: SockAddr = match domain {
			SockDomain::AfInet => unsafe {
				util::reinterpret::<SockAddrIn>(sockaddr)
			}.unwrap().clone().into(),

			SockDomain::AfInet6 => unsafe {
				util::reinterpret::<SockAddrIn6>(sockaddr)
			}.unwrap().clone().into(),

			_ => return Err(errno!(EPROTOTYPE)),
		};

		self.sockaddr = Some(sockaddr);

		// Opening connection if necessary
		match self.type_ {
			SockType::SockStream => {
				tcp::init_connection(self)?;
				self.state = SockState::WaitingAck;
			},

			SockType::SockSeqpacket => {
				// TODO
				todo!();
			},

			_ => self.state = SockState::Ready,
		}

		Ok(())
	}
}

impl FailableDefault for Socket {
	fn failable_default() -> Result<Self, Errno> {
		Ok(Self {
			domain: SockDomain::AfUnix,
			type_: SockType::SockRaw,
			protocol: 0,

			state: SockState::Created,
			sockaddr: None,

			receive_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),
			send_buffer: RingBuffer::new(crate::vec![0; BUFFER_SIZE]?),

			block_handler: BlockHandler::new(),
		})
	}
}

impl Buffer for Socket {
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
		_mem_space: IntSharedPtr<MemSpace>,
		_request: ioctl::Request,
		_argp: *const c_void,
	) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}

impl IO for Socket {
	fn get_size(&self) -> u64 {
		// TODO
		0
	}

	/// Note: This implemention ignores the offset.
	fn read(&mut self, _: u64, _buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		// TODO
		todo!();
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, buf: &[u8]) -> Result<u64, Errno> {
		let (
			transport,
			network,
			iface
		) = match self.domain {
			SockDomain::AfUnix => todo!(),

			dom @ (SockDomain::AfInet | SockDomain::AfInet6) => {
				let transport = match self.type_ {
					SockType::SockStream => TCPLayer {},
					SockType::SockDgram => todo!(), // TODO
					SockType::SockSeqpacket => todo!(), // TODO
					SockType::SockRaw => todo!(), // TODO
				};

				let network = match dom {
					SockDomain::AfInet => IPv4Layer {
						protocol: match self.type_ {
							SockType::SockStream => ip::PROTO_TCP,
							SockType::SockDgram => ip::PROTO_UDP,
							SockType::SockSeqpacket => todo!(), // TODO
							SockType::SockRaw => todo!(), // TODO
						},

						src_addr: [0; 4], // TODO
						dst_addr: [0; 4], // TODO
					},

					SockDomain::AfInet6 => todo!(), // TODO

					_ => unreachable!(),
				};

				// TODO use real dst addr
				let iface = net::get_iface_for(net::Address::IPv4([0; 4]));

				(
					Some(transport),
					network,
					iface
				)
			},

			SockDomain::AfNetlink => todo!(), // TODO

			SockDomain::AfPacket => todo!(), // TODO
		};

		network.transmit(buf.into(), |bufs| {
			let f = |bufs: BuffList<'_>| {
				let Some(ref iface_mutex) = iface else {
					return Ok(());
				};
				let mut iface = iface_mutex.lock();

				let buff = bufs.collect()?;
				iface.write(&buff)?;

				Ok(())
			};

			if let Some(ref transport) = transport {
				transport.transmit(bufs, f)
			} else {
				f(bufs)
			}
		})?;

		Ok(buf.len() as _)
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
