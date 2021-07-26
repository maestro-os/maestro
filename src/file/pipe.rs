//! A pipe is an object that links two file descriptors together. One reading and another writing,
//! with a buffer in between.

use crate::file::Errno;
use crate::util::container::vec::Vec;

/// The naximum size of a pipe's buffer.
const DEFAULT_BUFFER_SIZE: usize = 65536;

// TODO Implement ring buffer

/// Structure representing a pipe.
pub struct Pipe {
	/// The pipe's buffer.
	buffer: Vec<u8>,
}

impl Pipe {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		let mut s = Self {
			buffer: Vec::new(),
		};
		s.buffer.resize(DEFAULT_BUFFER_SIZE)?;

		Ok(s)
	}

	// TODO Function to get/set buffer size

	/// Reads data from the pipe.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, _buf: &mut [u8]) -> usize {
		// TODO
		todo!();
	}

	/// Writes data to the pipe.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, _buf: &[u8]) -> usize {
		// TODO
		todo!();
	}
}
