//! A pipe is an object that links two file descriptors together. One reading and another writing,
//! with a buffer in between.

use core::cmp::min;
use crate::file::Errno;
use crate::util::container::vec::Vec;

/// The naximum size of a pipe's buffer.
const BUFFER_SIZE: usize = 65536;

// TODO Implement ring buffer
// TODO Handle `limits::PIPE_BUF`

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
		s.buffer.resize(BUFFER_SIZE)?;

		Ok(s)
	}

	// TODO Function to get/set buffer size

	/// Reads data from the pipe.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, buf: &mut [u8]) -> usize {
		let len = min(buf.len(), self.buffer.len());
		for i in 0..len {
			buf[i] = self.buffer[0];
			self.buffer.remove(0);
		}

		len
	}

	/// Writes data to the pipe.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, buf: &[u8]) -> usize {
		let len = min(buf.len(), BUFFER_SIZE - self.buffer.len());
		for i in 0..len {
			// Won't crash because the memory is preallocated
			self.buffer.push(buf[i]).unwrap();
		}

		len
	}
}
