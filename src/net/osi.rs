//! The Open Systems Interconnection (OSI) model defines the architecure of a network stack.
//!
//! Such a stack is organized as layers.

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
