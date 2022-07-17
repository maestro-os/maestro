//! A pipe is an object that links two file descriptors together. One reading and another writing,
//! with a buffer in between.

use crate::file::Errno;
use crate::limits;
use crate::util::IO;
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

impl IO for PipeBuffer {
	fn get_size(&self) -> u64{
		self.get_data_len() as _
	}

	/// Note: This implemention ignores the offset.
	fn read(&mut self, _: u64, buf: &mut [u8]) -> Result<(u64, bool), Errno> {
		let len = self.buffer.read(buf);
		let eof = len == 0 && self.write_ends > 0;

		Ok((len as _, eof))
	}

	/// Note: This implemention ignores the offset.
	fn write(&mut self, _: u64, buf: &[u8]) -> Result<u64, Errno> {
		if self.read_ends > 0 {
			Ok(self.buffer.write(buf) as _)
		} else {
			Err(errno!(EPIPE))
		}
	}
}
