/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! A ring buffer is a data structure which allows to implement a FIFO
//! structure.
//!
//! The buffer works with a linear buffer and two cursors:
//! - The read cursor, which reads data until it reaches the write cursor
//! - The write cursor, which writes data until it reaches the read cursor
//!
//! When a cursor reaches the end of the linear buffer, it goes back to the
//! beginning. This is why it's called a "ring".

use core::{cmp::min, marker::PhantomData};

/// A ring buffer.
///
/// The ring buffer has a static size which is given at initialization.
///
/// The buffer used to store the data is specified by the generic argument `B`.
#[derive(Debug)]
pub struct RingBuffer<T, B: AsRef<[T]> + AsMut<[T]>> {
	/// The linear buffer.
	buffer: B,

	/// The offset of the read cursor in the buffer.
	read_cursor: usize,
	/// The offset of the write cursor in the buffer.
	write_cursor: usize,

	/// Allowing the argument T.
	_phantom: PhantomData<T>,
}

impl<T: Default + Copy, B: AsRef<[T]> + AsMut<[T]>> RingBuffer<T, B> {
	/// Creates a new instance.
	///
	/// `buffer` is the buffer to be used.
	pub fn new(buffer: B) -> Self {
		Self {
			buffer,

			read_cursor: 0,
			write_cursor: 0,

			_phantom: PhantomData,
		}
	}

	/// Returns the size of the buffer in number of elements.
	#[inline(always)]
	pub fn get_size(&self) -> usize {
		self.buffer.as_ref().len()
	}

	/// Tells whether the ring is empty.
	#[inline(always)]
	pub fn is_empty(&self) -> bool {
		self.read_cursor == self.write_cursor
	}

	/// Tells whether the ring is full.
	#[inline(always)]
	pub fn is_full(&self) -> bool {
		self.get_available_len() == 0
	}

	/// Returns the length of the data in the buffer.
	///
	/// If the buffer is empty, the function returns zero.
	pub fn get_data_len(&self) -> usize {
		if self.read_cursor <= self.write_cursor {
			self.write_cursor - self.read_cursor
		} else {
			self.get_size() - (self.read_cursor - self.write_cursor)
		}
	}

	/// Returns the length of the available space in the buffer.
	#[inline(always)]
	pub fn get_available_len(&self) -> usize {
		self.get_size() - self.get_data_len() - 1
	}

	/// Returns a slice representing the ring buffer's linear storage.
	#[inline(always)]
	fn get_buffer(&mut self) -> &mut [T] {
		self.buffer.as_mut()
	}

	/// Peeks dat afrom the buffer and writes it in `buf`.
	///
	/// Contrary to `read`, this function doesn't consume the data.
	///
	/// The function returns the number of elements read.
	pub fn peek(&mut self, buf: &mut [T]) -> usize {
		let cursor = self.read_cursor;
		let len = min(buf.len(), self.get_data_len());
		let buffer_size = self.get_size();
		let buffer = self.get_buffer();

		// The length of the first read, before going back to the beginning of the
		// buffer
		let l0 = min(cursor + len, buffer_size) - cursor;
		buf[..l0].copy_from_slice(&buffer[cursor..(cursor + l0)]);

		// The length of the second read, from the beginning of the buffer
		let l1 = len - l0;
		buf[l0..(l0 + l1)].copy_from_slice(&buffer[..l1]);

		len
	}

	/// Reads data from the buffer and writes it in `buf`.
	///
	/// The function returns the number of elements read.
	pub fn read(&mut self, buf: &mut [T]) -> usize {
		let len = self.peek(buf);
		let buffer_size = self.get_size();

		self.read_cursor = (self.read_cursor + len) % buffer_size;
		len
	}

	/// Writes data in `buf` to the buffer.
	///
	/// The function returns the number of elements written.
	pub fn write(&mut self, buf: &[T]) -> usize {
		let cursor = self.write_cursor;
		let len = min(buf.len(), self.get_available_len());
		let buffer_size = self.get_size();
		let buffer = self.get_buffer();

		// The length of the first read, before going back to the beginning of the
		// buffer
		let l0 = min(cursor + len, buffer_size) - cursor;
		buffer[cursor..(cursor + l0)].copy_from_slice(&buf[..l0]);

		// The length of the second read, from the beginning of the buffer
		let l1 = len - l0;
		buffer[..l1].copy_from_slice(&buf[l0..(l0 + l1)]);

		self.write_cursor = (self.write_cursor + len) % buffer_size;
		len
	}

	/// Clears the buffer.
	#[inline(always)]
	pub fn clear(&mut self) {
		// FIXME: Elements in the collections must be dropped here. However, using another
		// collections for storage might result in double dropping

		self.read_cursor = 0;
		self.write_cursor = 0;
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn ring_buffer0() {
		let mut rb = RingBuffer::new([0u8; 10]);
		let mut buf: [u8; 0] = [0; 0];
		assert_eq!(rb.read(&mut buf), 0);
	}

	#[test]
	fn ring_buffer1() {
		let mut rb = RingBuffer::new([0u8; 10]);
		let mut buf: [u8; 10] = [0; 10];
		assert_eq!(rb.read(&mut buf), 0);
	}

	#[test]
	fn ring_buffer2() {
		let mut rb = RingBuffer::new([0u8; 10]);

		let mut buf: [u8; 10] = [42; 10];
		assert_eq!(rb.write(&buf), 9);
		assert_eq!(rb.get_data_len(), 9);
		assert_eq!(rb.get_available_len(), 0);

		buf.fill(0);
		assert_eq!(rb.read(&mut buf), 9);
		assert_eq!(rb.get_data_len(), 0);
		assert_eq!(rb.get_available_len(), 9);

		for b in buf.iter().take(9) {
			assert_eq!(*b, 42);
		}
	}

	// TODO peek
}
