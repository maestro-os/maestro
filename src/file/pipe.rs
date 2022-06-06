//! A pipe is an object that links two file descriptors together. One reading and another writing,
//! with a buffer in between.

use crate::file::Errno;
use crate::limits;
use crate::util::container::ring_buffer::RingBuffer;

/// Structure representing a buffer buffer.
#[derive(Debug)]
pub struct PipeBuffer {
	/// The buffer's buffer.
	buffer: RingBuffer<u8>,

	/// The number of reading ends attached to the pipe.
	read_ends: u32,
	/// The number of writing ends attached to the pipe.
	write_ends: u32,
}

impl PipeBuffer {
	/// Creates a new instance.
	pub fn new() -> Result<Self, Errno> {
		Ok(Self {
			buffer: RingBuffer::new(limits::PIPE_BUF)?,

			read_ends: 0,
			write_ends: 0,
		})
	}

	/// Returns the length of the data to be read in the buffer.
	pub fn get_data_len(&self) -> usize {
		self.buffer.get_data_len()
	}

	/// Returns the available space in the buffer in bytes.
	pub fn get_available_len(&self) -> usize {
		self.buffer.get_available_len()
	}

	/// Reads data from the buffer.
	/// `buf` is the slice to write to.
	/// The functions returns the number of bytes that have been read.
	pub fn read(&mut self, buf: &mut [u8]) -> usize {
		self.buffer.read(buf)
	}

	/// Writes data to the buffer.
	/// `buf` is the slice to read from.
	/// The functions returns the number of bytes that have been written.
	pub fn write(&mut self, buf: &[u8]) -> Result<usize, Errno> {
		if self.read_ends > 0 {
			Ok(self.buffer.write(buf))
		} else {
			Err(errno!(EPIPE))
		}
	}

	/// Tells whether the EOF is reached for the pipe.
	pub fn eof(&self) -> bool {
		self.write_ends == 0
	}

	/// Updates the number of ends of the pipe.
	/// `write` tells whether the end is a writing end.
	/// `decrement` tells whether the decrement or increment the count.
	pub fn update_end_count(&mut self, write: bool, decrement: bool) {
		if decrement {
			if write {
				self.write_ends -= 1;
			} else {
				self.read_ends -= 1;
			}
		} else {

			if write {
				self.write_ends += 1;
			} else {
				self.read_ends += 1;
			}
		}
	}
}
