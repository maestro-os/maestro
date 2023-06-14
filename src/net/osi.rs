//! The Open Systems Interconnection (OSI) model defines the architecure of a network stack.

use super::buff::BuffList;
use crate::errno::Errno;

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

/*/// TODO doc
pub struct Stack {
	/// TODO doc
	network: Box<dyn Layer>,
	/// TODO doc
	transport: Box<dyn Layer>,
}

/// The list of registered stacks.
static STACKS: Mutex<HashMap<SocketDesc, Vec<Box<dyn Layer>>>> = Mutex::new(HashMap::new());

/// Returns the stack for the given socket descriptor.
///
/// If no stack match, the function returns `None`.
pub fn stack_for(desc: &SocketDesc) -> Option<Stack> {
	// TODO
	// - domain is layer 3 of the OSI model (network)
	// - type determines the way the socket communicates with the userspace
	// - protocol is the protocol to use on layer 4 (transport). If not specified, the default protocol for the domain/type pair is used
	todo!()
}*/
