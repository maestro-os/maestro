//! The Open Systems Interconnection (OSI) model defines the architecure of a network stack.

use super::{buff::BuffList, ip, SocketDesc, SocketDomain, SocketType};
use crate::{
	errno::Errno,
	util::{boxed::Box, collections::hashmap::HashMap, lock::Mutex},
};

/// An OSI layer.
///
/// A layer stack acts as a pipeline, passing data from one layer to the other.
pub trait Layer {
	// TODO receive

	/// Transmits data in the given buffer.
	///
	/// Arguments:
	/// - `buff` is the list of buffer which composes the packet being built.
	/// - `next` is the function called to pass the buffers list to the next layer.
	fn transmit<'c, F>(&self, buff: BuffList<'c>, next: F) -> Result<(), Errno>
	where
		Self: Sized,
		F: Fn(BuffList<'c>) -> Result<(), Errno>;
}

/// Function used to build a layer from a given sockaddr structure.
pub type LayerBuilder = fn(&[u8]) -> Result<Box<dyn Layer>, Errno>;

/// Collection of OSI layers 3 (network)
static DOMAINS: Mutex<HashMap<u32, LayerBuilder>> = Mutex::new(HashMap::new());
/// Collection of OSI layers 4 (transport)
static PROTOCOLS: Mutex<HashMap<u32, LayerBuilder>> = Mutex::new(HashMap::new());

/// Collection of default protocols ID for domain/type pairs.
///
/// If this collection doesn't contain a pair, it is considered invalid.
static DEFAULT_PROTOCOLS: Mutex<HashMap<(u32, SocketType), u32>> = Mutex::new(HashMap::new());

/// A stack of layers for a socket.
pub struct Stack {
	/// The socket's protocol on OSI layer 3.
	pub domain: Box<dyn Layer>,
	/// The socket's protocol on OSI layer 4.
	pub protocol: Box<dyn Layer>,
}

impl Stack {
	/// Creates a new socket network stack.
	///
	/// Arguments:
	/// - `desc` is the descriptor of the socket.
	/// - `sockaddr` is the socket address structure containing informations to initialize the
	/// stack.
	///
	/// If the descriptor is invalid or if the stack cannot be created, the function returns an
	/// error.
	pub fn new(desc: &SocketDesc, sockaddr: &[u8]) -> Result<Stack, Errno> {
		let domain = {
			let guard = DOMAINS.lock();
			let builder = guard
				.get(&desc.domain.get_id())
				.ok_or_else(|| errno!(EINVAL))?;
			builder(sockaddr)?
		};

		let protocol: u32 = if desc.protocol != 0 {
			desc.protocol as _
		} else {
			*DEFAULT_PROTOCOLS
				.lock()
				.get(&(desc.domain.get_id(), desc.type_))
				.ok_or_else(|| errno!(EINVAL))?
		};
		let protocol = {
			let guard = PROTOCOLS.lock();
			let builder = guard.get(&protocol).ok_or_else(|| errno!(EINVAL))?;
			builder(sockaddr)?
		};

		Ok(Stack {
			domain,
			protocol,
		})
	}
}

/// Registers default domains/types/protocols.
pub(crate) fn init() -> Result<(), Errno> {
	let domains = HashMap::try_from([
		// TODO unix
		(
			SocketDomain::AfInet.get_id(),
			ip::inet_build as LayerBuilder,
		),
		(
			SocketDomain::AfInet6.get_id(),
			ip::inet6_build as LayerBuilder,
		),
		// TODO netlink
		// TODO packet
	])?;
	let protocols = HashMap::try_from([
		// TODO tcp
		// TODO udp
	])?;
	let default_protocols = HashMap::try_from([
		// TODO unix

		// ((SocketDomain::AfInet.get_id(), SocketType::SockStream.get_id()), /* TODO: ipv4/tcp */),
		// ((SocketDomain::AfInet.get_id(), SocketType::SockDgram.get_id()), /* TODO: ipv4/udp */),

		// ((SocketDomain::AfInet6.get_id(), SocketType::SockStream.get_id()), /* TODO: ipv6/tcp */),
		// ((SocketDomain::AfInet6.get_id(), SocketType::SockDgram.get_id()), /* TODO: ipv6/udp */),

		// TODO netlink
		// TODO packet
	])?;

	*DOMAINS.lock() = domains;
	*PROTOCOLS.lock() = protocols;
	*DEFAULT_PROTOCOLS.lock() = default_protocols;

	Ok(())
}
