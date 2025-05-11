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

//! Ring buffer implementation.

use crate::memory::{
	malloc::{__alloc, __dealloc},
	user::UserSlice,
};
use core::{alloc::Layout, cmp::min, num::NonZeroUsize, ptr::NonNull};
use utils::errno::{AllocResult, EResult};

/// Ring buffer of `u8`.
#[derive(Debug)]
pub struct RingBuffer {
	/// The linear, allocated buffer
	buf: NonNull<[u8]>,
	/// The size of the buffer in bytes
	capacity: NonZeroUsize,

	/// The offset of the read cursor in the buffer
	read_cursor: usize,
	/// The offset of the write cursor in the buffer
	write_cursor: usize,
}

impl RingBuffer {
	/// Creates a new instance.
	///
	/// `capacity` is the size of the buffer in bytes.
	pub fn new(capacity: NonZeroUsize) -> AllocResult<Self> {
		let layout = Layout::array::<u8>(capacity.get()).unwrap();
		let buf = unsafe { __alloc(layout)? };
		Ok(Self {
			buf,
			capacity,

			read_cursor: 0,
			write_cursor: 0,
		})
	}

	/// Returns the size of the buffer in bytes.
	#[inline(always)]
	pub fn capacity(&self) -> usize {
		self.capacity.get()
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
			self.capacity() - (self.read_cursor - self.write_cursor)
		}
	}

	/// Returns the length of the available space in the buffer.
	#[inline(always)]
	pub fn get_available_len(&self) -> usize {
		self.capacity() - self.get_data_len() - 1
	}

	/// Returns a slice representing the ring buffer's linear storage.
	#[inline(always)]
	fn inner_buffer(&mut self) -> &mut [u8] {
		unsafe { self.buf.as_mut() }
	}

	/// Peeks data from the buffer and writes it in `buf`.
	///
	/// Contrary to `read`, this function does not consume the data.
	///
	/// The function returns the number of bytes read.
	pub fn peek(&mut self, buf: UserSlice<u8>) -> EResult<usize> {
		let cursor = self.read_cursor;
		let len = min(buf.len(), self.get_data_len());
		let capacity = self.capacity();
		let buffer = self.inner_buffer();
		// First read
		let l0 = min(cursor + len, capacity) - cursor;
		buf.copy_to_user(0, &buffer[cursor..(cursor + l0)])?;
		// Second read
		let l1 = len - l0;
		buf.copy_to_user(l0, &buffer[..l1])?;
		Ok(len)
	}

	/// Reads data from the buffer and writes it in `buf`.
	///
	/// The function returns the number of bytes read.
	pub fn read(&mut self, buf: UserSlice<u8>) -> EResult<usize> {
		let len = self.peek(buf)?;
		self.read_cursor = (self.read_cursor + len) % self.capacity();
		Ok(len)
	}

	/// Writes data in `buf` to the buffer.
	///
	/// The function returns the number of bytes written.
	pub fn write(&mut self, buf: UserSlice<u8>) -> EResult<usize> {
		let cursor = self.write_cursor;
		let len = min(buf.len(), self.get_available_len());
		let capacity = self.capacity();
		let buffer = self.inner_buffer();
		// First write
		let l0 = min(cursor + len, capacity) - cursor;
		buf.copy_from_user(0, &mut buffer[cursor..(cursor + l0)])?;
		// Second write
		let l1 = len - l0;
		buf.copy_from_user(l0, &mut buffer[..l1])?;
		// Update cursor
		self.write_cursor = (self.write_cursor + len) % capacity;
		Ok(len)
	}

	/// Clears the buffer.
	#[inline(always)]
	pub fn clear(&mut self) {
		self.read_cursor = 0;
		self.write_cursor = 0;
	}
}

impl Drop for RingBuffer {
	fn drop(&mut self) {
		// Free buffer
		let layout = Layout::array::<u8>(self.capacity.get()).unwrap();
		unsafe {
			__dealloc(self.buf.cast(), layout);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn ring_buffer_read() {
		let mut rb = RingBuffer::new(NonZeroUsize::new(10).unwrap()).unwrap();
		let mut buf: [u8; 0] = [0; 0];
		let len = rb.read(UserSlice::from_slice_mut(&mut buf)).unwrap();
		assert_eq!(len, 0);

		let mut buf: [u8; 10] = [0; 10];
		let len = rb.read(UserSlice::from_slice_mut(&mut buf)).unwrap();
		assert_eq!(len, 0);
	}

	#[test_case]
	fn ring_buffer_write() {
		let mut rb = RingBuffer::new(NonZeroUsize::new(10).unwrap()).unwrap();

		let mut buf: [u8; 10] = [42; 10];
		let len = rb.write(UserSlice::from_slice_mut(&mut buf)).unwrap();
		assert_eq!(len, 9);
		assert_eq!(rb.get_data_len(), 9);
		assert_eq!(rb.get_available_len(), 0);

		buf.fill(0);
		let len = rb.read(UserSlice::from_slice_mut(&mut buf)).unwrap();
		assert_eq!(len, 9);
		assert_eq!(rb.get_data_len(), 0);
		assert_eq!(rb.get_available_len(), 9);

		for b in buf.iter().take(9) {
			assert_eq!(*b, 42);
		}
	}

	// TODO peek
}
