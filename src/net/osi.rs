//! The Open Systems Interconnection (OSI) model defines the architecure of a network stack.

use crate::errno::Errno;
use crate::util::container::hashmap::HashMap;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::ptr::arc::Weak;
use super::SocketDesc;
use super::SocketDomain;
use super::SocketType;
use super::buff::BuffList;

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

/// Container of OSI layers 3 (network)
pub static DOMAINS: Mutex<HashMap<u32, Arc<dyn Layer>>> = Mutex::new(HashMap::new());
/// Container of OSI layers 4 (transport)
pub static PROTOCOLS: Mutex<HashMap<u32, Arc<dyn Layer>>> = Mutex::new(HashMap::new());

/// Container of default protocols for domain/type pairs.
///
/// If this container doesn't contain a pair, it is considered invalid.
pub static DEFAULT_PROTOCOLS: Mutex<HashMap<(u32, u32), Arc<dyn Layer>>> =
	Mutex::new(HashMap::new());

/// A stack of layers for a socket.
pub struct Stack {
	/// The socket's protocol on OSI layer 3.
	pub domain: Weak<dyn Layer>,
	/// The socket's protocol on OSI layer 4.
	pub protocol: Weak<dyn Layer>,
}

/// Returns the stack for the given socket descriptor.
///
/// If the descriptor is invalid, the function returns `None`.
pub fn get_stack(desc: &SocketDesc) -> Option<Stack> {
	let domain = {
		let guard = DOMAINS.lock();
		let arc = guard.get(&desc.domain.get_id())?;
		Arc::downgrade(arc)
	};
	let protocol = if desc.protocol != 0 {
		let guard = PROTOCOLS.lock();
		let arc = guard.get(&(desc.protocol as _))?;
		Arc::downgrade(arc)
	} else {
		let guard = DEFAULT_PROTOCOLS.lock();
		let arc = guard.get(&(desc.domain.get_id(), desc.type_.get_id()))?;
		Arc::downgrade(arc)
	};

	Some(Stack {
		domain,
		protocol,
	})
}

/// Registers default domains/types/protocols.
pub fn init() -> Result<(), Errno> {
	*DOMAINS.lock() = HashMap::try_from([
		// TODO unix
		// TODO (SocketDomain::AfInet.get_id(), Arc::new(Ipv4Layer {})),
		// TODO (SocketDomain::AfInet6.get_id(), Arc::new(Ipv6Layer {})),
		// TODO netlink
		// TODO packet
	])?;

	*PROTOCOLS.lock() = HashMap::try_from([
		// TODO tcp
		// TODO udp
	])?;

	*DEFAULT_PROTOCOLS.lock() = HashMap::try_from([
		// TODO unix

		// ((SocketDomain::AfInet.get_id(), SocketType::SockStream.get_id()), /* TODO: ipv4/tcp */),
		// ((SocketDomain::AfInet.get_id(), SocketType::SockDgram.get_id()), /* TODO: ipv4/udp */),

		// ((SocketDomain::AfInet6.get_id(), SocketType::SockStream.get_id()), /* TODO: ipv6/tcp */),
		// ((SocketDomain::AfInet6.get_id(), SocketType::SockDgram.get_id()), /* TODO: ipv6/udp */),

		// TODO netlink
		// TODO packet
	])?;

	Ok(())
}
