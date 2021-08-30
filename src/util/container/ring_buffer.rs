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
		buffer.resize(size + 1)?;

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
		self.get_size() - self.get_data_len() - 1
	}

	/// Returns a slice representing the ring buffer's linear buffer.
	#[inline(always)]
	fn get_buffer(&mut self) -> &mut [u8] {
		self.buffer.as_mut_slice()
	}

	/// Reads data from the buffer and writes it in `buf`. The function returns the number of bytes
	/// read.
	pub fn read(&mut self, buf: &mut [u8]) -> usize {
		let cursor = self.read_cursor;
		let len = min(buf.len(), self.get_data_len());

		let buffer_size = self.get_size();
		let buffer = self.get_buffer();

		// The length of the first read, before going back to the beginning of the buffer
		let l0 = min(cursor + len, buffer_size) - cursor;
		for i in 0..l0 {
			buf[i] = buffer[cursor + i];
		}

		// The length of the second read, from the beginning of the buffer
		let l1 = len - l0;
		for i in 0..l1 {
			buf[l0 + i] = buffer[i];
		}

		self.read_cursor = (self.read_cursor + len) % buffer_size;
		len
	}

	/// Writes data in `buf` to the buffer. The function returns the number of bytes written.
	pub fn write(&mut self, buf: &[u8]) -> usize {
		let cursor = self.write_cursor;
		let len = min(buf.len(), self.get_available_len());

		let buffer_size = self.get_size();
		let buffer = self.get_buffer();

		// The length of the first read, before going back to the beginning of the buffer
		let l0 = min(cursor + len, buffer_size) - cursor;
		for i in 0..l0 {
			buffer[cursor + i] = buf[i];
		}

		// The length of the second read, from the beginning of the buffer
		let l1 = len - l0;
		for i in 0..l1 {
			buffer[i] = buf[l0 + i];
		}

		self.write_cursor = (self.write_cursor + len) % buffer_size;
		len
	}

	/// Clears the buffer.
	#[inline(always)]
	pub fn clear(&mut self) {
		self.read_cursor = 0;
		self.write_cursor = 0;
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn ring_buffer0() {
		let mut rb = RingBuffer::new(10).unwrap();
		let mut buf: [u8; 0] = [0; 0];
		assert_eq!(rb.read(&mut buf), 0);
	}

	#[test_case]
	fn ring_buffer1() {
		let mut rb = RingBuffer::new(10).unwrap();
		let mut buf: [u8; 10] = [0; 10];
		assert_eq!(rb.read(&mut buf), 0);
	}

	#[test_case]
	fn ring_buffer2() {
		let mut rb = RingBuffer::new(10).unwrap();
		let mut buf: [u8; 10] = [0; 10];
		for i in 0..buf.len() {
			buf[i] = 42;
		}

		assert_eq!(rb.write(&buf), 10);
		assert_eq!(rb.get_data_len(), 10);
		assert_eq!(rb.get_available_len(), 0);

		for i in 0..buf.len() {
			buf[i] = 0;
		}

		assert_eq!(rb.read(&mut buf), 10);
		assert_eq!(rb.get_data_len(), 0);
		assert_eq!(rb.get_available_len(), 10);

		for i in 0..buf.len() {
			assert_eq!(buf[i], 42);
		}
	}

	// TODO More tests
}
