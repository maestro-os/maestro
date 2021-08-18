//! A ring buffer is a data structure which allows to implement a FIFO structure.
//!
//! The buffer works with a linear buffer and two cursors:
//! - The read cursor, which reads data until it reaches the write cursor
//! - The write cursor, which writes data until it reaches the read cursor
//!
//! When a cursor reaches the end of the linear buffer, it goes back to the beginning. This is why
//! it's called a "ring".

use core::cmp::min;
use crate::errno::Errno;
use crate::util::container::vec::Vec;

/// Structure representing a ring buffer. The buffer has a limited size which must be given at
/// initialization.
pub struct RingBuffer {
	/// The linear buffer.
	buffer: Vec<u8>,

	/// The offset of the read cursor in the buffer.
	read_cursor: usize,
	/// The offset of the write cursor in the buffer.
	write_cursor: usize,
}

impl RingBuffer {
	/// Creates a new instance.
	/// `size` is the size of the buffer.
	pub fn new(size: usize) -> Result<Self, Errno> {
		let mut buffer = Vec::<u8>::new();
		buffer.resize(size)?;

		Ok(Self {
			buffer,

			read_cursor: 0,
			write_cursor: 0,
		})
	}

	/// Returns the size of the buffer in bytes.
	#[inline(always)]
	pub fn get_size(&self) -> usize {
		self.buffer.len()
	}

	/// Tells whether the ring is empty.
	#[inline(always)]
	pub fn is_empty(&self) -> bool {
		self.read_cursor == self.write_cursor
	}

	/// Returns the length in bytes of the data in the buffer. If the buffer is empty, the function
	/// returns zero.
	pub fn get_data_len(&self) -> usize {
		if self.read_cursor <= self.write_cursor {
			self.write_cursor - self.read_cursor
		} else {
			self.get_size() - (self.read_cursor - self.write_cursor)
		}
	}

	/// Returns the length of the available space in bytes in the buffer.
	#[inline(always)]
	pub fn get_available_len(&self) -> usize {
		self.get_size() - self.get_data_len()
	}

	/// Reads data from the buffer and writes it in `buf`. The function returns the number of bytes
	/// read.
	pub fn read(&mut self, buf: &mut [u8]) -> usize {
		let _len = min(buf.len(), self.get_data_len());

		// TODO
		todo!();
	}

	/// Writes data in `buf` to the buffer. The function returns the number of bytes written.
	pub fn write(&mut self, buf: &[u8]) -> usize {
		let _len = min(buf.len(), self.get_available_len());

		// TODO
		todo!();
	}

	/// Clears the buffer.
	#[inline(always)]
	pub fn clear(&mut self) {
		self.read_cursor = 0;
		self.write_cursor = 0;
	}
}
