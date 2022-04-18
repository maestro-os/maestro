//! A pipe is an object that links two file descriptors together. One reading and another writing,
//! with a buffer in between.

use crate::file::Errno;
use crate::util::container::ring_buffer::RingBuffer;

/// The maximum size of a pipe's buffer.
const BUFFER_SIZE: usize = 65536;

// TODO Handle `limits::PIPE_BUF`

/// Structure representing a pipe.
pub struct Pipe {
	/// The reading file descriptor.
	fd0: u32,
	/// The writing file descriptor.
	fd1: u32,

	/// The pipe's buffer.
	buffer: RingBuffer<u8>,

	/// Tells whether the pipe is closed.
	closed: bool,
}

impl Pipe {
	/// Creates a new instance.
	pub fn new(fd0: u32, fd1: u32) -> Result<Self, Errno> {
		Ok(Self {
			fd0,
			fd1,

			buffer: RingBuffer::new(BUFFER_SIZE)?,

			closed: false,
		})
	}

	/// Returns the file descriptor at the reading end of the pipe.
	pub fn get_fd0(&self) -> u32 {
		self.fd0
	}

	/// Sets the file descriptor at the reading end of the pipe.
	pub fn set_fd0(&mut self, fd0: u32) {
		self.fd0 = fd0;
	}

	/// Returns the file descriptor at the writing end of the pipe.
	pub fn get_fd1(&self) -> u32 {
		self.fd1
	}

	/// Sets the file descriptor at the writing end of the pipe.
	pub fn set_fd1(&mut self, fd1: u32) {
		self.fd1 = fd1;
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
