//! This module stores the Bitfield structure.

use crate::errno::Errno;
use crate::util::bit_size_of;
use crate::util::container::vec::Vec;
use crate::util::math::ceil_division;

/// A bitfield is a data structure meant to contain only boolean values.
/// The size of the bitfield is specified at initialization.
pub struct Bitfield {
	/// The bitfield's data.
	data: Vec<u8>,
	/// The number of bits in the bitfield.
	len: usize,
	/// The number of set bits.
	set_count: usize,
}

impl Bitfield {
	/// Creates a new bitfield with the given number of bits `len`.
	pub fn new(len: usize) -> Result<Self, Errno> {
		let size = ceil_division(len, bit_size_of::<u8>());

		let mut bitfield = Self {
			data: Vec::with_capacity(size)?,
			len,
			set_count: 0,
		};
		for _ in 0..size {
			bitfield.data.push(0)?;
		}
		Ok(bitfield)
	}

	/// Returns the number of bit in the bitfield.
	pub fn len(&self) -> usize {
		self.len
	}

	/// Returns the size of the memory region of the bitfield in bytes.
	pub fn mem_size(&self) -> usize {
		ceil_division(self.len, bit_size_of::<u8>())
	}

	/// Returns the number of set bits.
	pub fn set_count(&self) -> usize {
		self.set_count
	}

	/// Tells whether bit `index` is set.
	pub fn is_set(&self, index: usize) -> bool {
		let unit = self.data[(index / bit_size_of::<u8>()) as _];
		(unit >> (index % bit_size_of::<u8>())) & 1 == 1
	}

	/// Sets bit `index`.
	pub fn set(&mut self, index: usize) {
		debug_assert!(index < self.len);

		if !self.is_set(index) {
			let unit = &mut self.data[(index / bit_size_of::<u8>()) as _];
			*unit |= 1 << (index % bit_size_of::<u8>());

			self.set_count += 1;
		}
	}

	/// Clears bit `index`.
	pub fn clear(&mut self, index: usize) {
		debug_assert!(index < self.len);

		if self.is_set(index) {
			let unit = &mut self.data[(index / bit_size_of::<u8>()) as _];
			*unit &= !(1 << (index % bit_size_of::<u8>()));

			self.set_count -= 1;
		}
	}

	/// Finds a clear bit. The function returns the offset to the bit. If none is found, the
	/// function returns None.
	pub fn find_clear(&self) -> Option<usize> {
		for i in 0..self.len {
			if !self.is_set(i) {
				return Some(i);
			}
		}

		None
	}

	/// Finds a set bit. The function returns the offset to the bit. If none is found, the function
	/// returns None.
	pub fn find_set(&self) -> Option<usize> {
		for i in 0..self.len {
			if self.is_set(i) {
				return Some(i);
			}
		}

		None
	}

	/// Clears every elements in the bitfield.
	pub fn clear_all(&mut self) {
		for i in 0..self.data.len() {
			self.data[i] = 0;
		}
	}

	/// Clears every elements in the bitfield.
	pub fn set_all(&mut self) {
		for i in 0..self.data.len() {
			self.data[i] = !0;
		}
	}

	// TODO fill
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn bitfield_set0() {
		let mut bitfield = Bitfield::new(42).unwrap();
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

	#[test_case]
	fn bitfield_clear0() {
		let mut bitfield = Bitfield::new(42).unwrap();
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
