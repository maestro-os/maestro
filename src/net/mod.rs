//! This module implements the network stack.

pub mod icmp;
pub mod ip;
pub mod lo;

use crate::errno::Errno;
use crate::util::boxed::Box;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;
use crate::util::lock::Mutex;

/// The size of the receive ring buffer for each interface.
const RX_BUFF_SIZE: usize = 4096;

/// Type representing a Media Access Control (MAC) address.
pub type MAC = [u8; 6];

/// An enumeration of network address types.
pub enum Address {
	/// Internet Protocol version 4.
	IPv4([u8; 4]),
	/// Internet Protocol version 6.
	IPv6([u8; 16]),
}

/// An address/subnet mask pair to be bound to an interface.
pub struct BindAddress {
	/// The bound address.
	pub addr: Address,
	/// Subnet mask/prefix length.
	pub subnet_mask: u8,
}

/// Trait representing a network interface.
pub trait Interface {
	/// Returns the name of the interface.
	fn get_name(&self) -> &[u8];

	/// Tells whether the interface is UP.
	fn is_up(&self) -> bool;

	/// Returns the mac address of the interface.
	fn get_mac(&self) -> &MAC;

	/// Returns the list of addresses bound to the interface.
	fn get_addresses(&self) -> &[BindAddress];

	/// Reads data from the network interface and writes it into `buff`.
	/// The function returns the number of bytes read.
	fn read(&mut self, buff: &mut [u8]) -> Result<(u64, bool), Errno>;

	/// Reads data from `buff` and writes it into the network interface.
	/// The function returns the number of bytes written.
	fn write(&mut self, buff: &[u8]) -> Result<u64, Errno>;
}

/// Trait implemented on a type used as output of a read operation on a layer.
pub trait LayerReadOut<'a> {}
/// Trait implemented on a type used as output of a write operation on a layer.
pub trait LayerWriteOut<'a> {}

/// Trait representing a layer of the network stack.
pub trait Layer {
	/// Input of the layer on a read operation.
	type ReadIn;
	/// Output of the layer on a read operation.
	type ReadOut<'a>: LayerReadOut<'a>;

	/// Input of the layer on a write operation.
	type WriteIn;
	/// Output of the layer on a write operation.
	type WriteOut<'a>: LayerWriteOut<'a>;

	/// Consumes data from the given input and yield another input for the next layer.
	/// This function is used to receive data from a network interface.
	/// If no input results from the layer, the function returns None.
	/// The function may choose the consume data without returning anything, thus discarding the
	/// data.
	fn consume_input<'a>(&self, val: &'a mut Self::ReadIn) -> Option<Self::ReadOut<'a>>;

	/// Creates an output to be passed to the next layer.
	/// This function is used to transmit data on a network interface.
	fn create_output<'a>(&self, val: &'a Self::WriteIn) -> Self::WriteOut<'a>;
}

/// A link layer.
pub struct LinkLayer {
	/// The network interface.
	iface: Box<dyn Interface>,

	/// The ring buffer used to receive packets.
	rx_buff: RingBuffer<u8>,
}

impl LinkLayer {
	/// Creates a new instance.
	pub fn new(iface: Box<dyn Interface>) -> Result<Self, Errno> {
		Ok(Self {
			iface,

			rx_buff: RingBuffer::new(RX_BUFF_SIZE)?,
		})
	}
}

/// The current list of interfaces. The container stores link layers, one for each interface.
static INTERFACES: Mutex<Vec<LinkLayer>> = Mutex::new(Vec::new());
