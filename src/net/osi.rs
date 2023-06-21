//! The Open Systems Interconnection (OSI) model defines the architecure of a network stack.

use super::buff::BuffList;
use super::SocketDesc;
use crate::errno::Errno;
use crate::util::container::hashmap::HashMap;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;

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
pub static PROTOCOLS: Mutex<HashMap<u32, Arc<dyn Layer>>> = Mutex::new(HashMap::new());
/// Container of OSI layers 4 (transport)
pub static TRANSPORTS: Mutex<HashMap<u32, Arc<dyn Layer>>> = Mutex::new(HashMap::new());

/// Container of default protocols for domain/transport pairs.
///
/// If this container doesn't contain a pair, it is considered invalid.
pub static DEFAULT_PROTOCOLS: Mutex<HashMap<(u32, u32), Arc<dyn Layer>>> =
	Mutex::new(HashMap::new());

// TODO: layers must be customizable (sockaddr info)

/// A stack of layers for a socket.
pub struct Stack {
	/// The socket's protocol on OSI layer 3.
	pub protocol: Arc<dyn Layer>,
	/// The socket's protocol on OSI layer 4.
	pub transport: Arc<dyn Layer>,
}

/// Returns the stack for the given socket descriptor.
///
/// If the descriptor is invalid, the function returns `None`.
pub fn get_stack(desc: &SocketDesc) -> Option<Stack> {
	let protocol = if desc.protocol != 0 {
		PROTOCOLS.lock().get(&(desc.protocol as _))?.clone()
	} else {
		DEFAULT_PROTOCOLS
			.lock()
			.get(&(desc.domain.get_id(), desc.type_.get_id()))?
			.clone()
	};
	let transport = TRANSPORTS.lock().get(&desc.type_.get_id())?.clone();

	Some(Stack {
		protocol,
		transport,
	})
}
