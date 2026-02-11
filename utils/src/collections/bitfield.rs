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

//! Bitfield implementation.

use crate::{TryClone, collections::vec::Vec, errno::AllocResult};
use core::hint::likely;

/// A bitfield is a data structure meant to contain only boolean values.
///
/// The size of the bitfield is specified at initialization.
pub struct Bitfield<C: AsRef<[u8]> + AsMut<[u8]> = Vec<u8>> {
	data: C,
	/// The number of bits in the bitfield
	len: usize,
}

impl Bitfield<Vec<u8>> {
	/// Creates a new bitfield, allocated on the heap, with the given number of bits
	pub fn new_allocated(len: usize) -> AllocResult<Self> {
		let size = len.div_ceil(8);
		let bitfield = Self {
			data: crate::vec![0; size]?,
			len,
		};
		Ok(bitfield)
	}
}

impl<const N: usize> Bitfield<[u8; N]> {
	/// Creates a new bitfield, stored in place
	///
	/// The length is `N * 8`
	pub const fn new_inplace() -> Self {
		Self {
			data: [0; N],
			len: N * 8,
		}
	}
}

impl<C: AsRef<[u8]> + AsMut<[u8]>> Bitfield<C> {
	/// Returns the number of bit in the bitfield.
	#[inline]
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.len
	}

	/// Tells whether bit `index` is set.
	#[inline]
	pub fn is_set(&self, index: usize) -> bool {
		let unit = self.data.as_ref()[index / u8::BITS as usize];
		(unit >> (index % u8::BITS as usize)) & 1 == 1
	}

	/// Sets bit `index`.
	pub fn set(&mut self, index: usize) {
		debug_assert!(index < self.len);
		if !self.is_set(index) {
			let unit = &mut self.data.as_mut()[index / u8::BITS as usize];
			*unit |= 1 << (index % u8::BITS as usize);
		}
	}

	/// Clears bit `index`.
	pub fn clear(&mut self, index: usize) {
		debug_assert!(index < self.len);
		if self.is_set(index) {
			let unit = &mut self.data.as_mut()[index / u8::BITS as usize];
			*unit &= !(1 << (index % u8::BITS as usize));
		}
	}

	/// Finds a set bit.
	///
	/// The function returns the offset to the bit.
	///
	/// If none is found, the function returns `None`.
	pub fn find_set(&self) -> Option<usize> {
		// TODO optimize (using mask)
		(0..self.len).find(|i| self.is_set(*i))
	}

	/// Finds a clear bit.
	///
	/// The function returns the offset to the bit.
	///
	/// If none is found, the function returns `None`.
	pub fn find_clear(&self) -> Option<usize> {
		// TODO optimize (using mask)
		(0..self.len).find(|i| !self.is_set(*i))
	}

	/// Clears every element in the bitfield.
	pub fn clear_all(&mut self) {
		self.data.as_mut().fill(0);
	}

	/// Clears every element in the bitfield.
	pub fn set_all(&mut self) {
		self.data.as_mut().fill(!0);
	}

	/// Returns an immutable iterator over the bitfield.
	pub fn iter(&self) -> BitfieldIterator<C> {
		BitfieldIterator {
			bitfield: self,
			cursor: 0,
		}
	}
}

impl TryClone for Bitfield {
	fn try_clone(&self) -> AllocResult<Self> {
		Ok(Self {
			data: self.data.try_clone()?,
			len: self.len,
		})
	}
}

/// An immutable iterator over a bitfield.
pub struct BitfieldIterator<'b, C: AsRef<[u8]> + AsMut<[u8]>> {
	/// The bitfield.
	bitfield: &'b Bitfield<C>,
	/// The cursor of the iterator.
	cursor: usize,
}

impl<C: AsRef<[u8]> + AsMut<[u8]>> Iterator for BitfieldIterator<'_, C> {
	type Item = bool;

	fn next(&mut self) -> Option<Self::Item> {
		if likely(self.cursor < self.bitfield.len()) {
			let val = self.bitfield.is_set(self.cursor);
			self.cursor += 1;
			Some(val)
		} else {
			None
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn bitfield_set0() {
		let mut bitfield = Bitfield::new_allocated(42).unwrap();
		assert_eq!(bitfield.len(), 42);

		for i in 0..bitfield.len() {
			assert!(!bitfield.is_set(i));
		}

		for i in 0..bitfield.len() {
			bitfield.set(i);
		}

		for i in 0..bitfield.len() {
			assert!(bitfield.is_set(i));
		}
	}

	#[test]
	fn bitfield_clear0() {
		let mut bitfield = Bitfield::new_allocated(42).unwrap();
		assert_eq!(bitfield.len(), 42);

		for i in 0..bitfield.len() {
			bitfield.set(i);
		}

		for i in 0..bitfield.len() {
			assert!(bitfield.is_set(i));
		}

		for i in 0..bitfield.len() {
			bitfield.clear(i);
		}

		for i in 0..bitfield.len() {
			assert!(!bitfield.is_set(i));
		}
	}

	// TODO Write more tests
}
