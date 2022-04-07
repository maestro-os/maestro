//! A pipe is an object that links two file descriptors together. One reading and another writing,
//! with a buffer in between.

use crate::file::Errno;
use crate::util::container::ring_buffer::RingBuffer;

/// The maximum size of a pipe's buffer.
const BUFFER_SIZE: usize = 65536;

// TODO Handle `limits::PIPE_BUF`

/// Structure representing a pipe.
pub struct Pipe {
	/// The pipe's buffer.
	buffer: RingBuffer<u8>,

	/// Tells whether the pipe is closed.
	closed: bool,
}

impl Pipe {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			buffer: RingBuffer::new(BUFFER_SIZE)?,

			closed: false,
		})
	}

	// TODO Function to get/set buffer size

	/// Returns the available space in the pipe in bytes.
	pub fn get_available_len(&self) -> usize {
		self.buffer.get_available_len()
	}

	/// Reads data from the pipe.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, buf: &mut [u8]) -> usize {
		self.buffer.read(buf)
	}

	/// Writes data to the pipe.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, buf: &[u8]) -> Result<usize, Errno> {
		if !self.closed {
			Ok(self.buffer.write(buf))
		} else {
			Err(errno!(EPIPE))
		}
	}

	/// Tells whether the pipe is closed.
	pub fn is_closed(&self) -> bool {
		self.closed
	}

	/// Closes the pipe. If the pipe is already closed, the function does nothing.
	pub fn close(&mut self) {
		self.closed = true;
	}
}
