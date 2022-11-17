//! This module implements a generic iterator to be used on tables in ELF files.

use core::marker::PhantomData;
use crate::util;

/// A generic iterator for ELF tables.
///
/// `T` is the type of the structure contained in the table.
pub struct ELFIterator<'a, T> {
	/// A reference to the table's memory.
	table: &'a [u8],
	/// The size in bytes of an entry in the table.
	entsize: usize,

	/// The current offset in the table.
	curr_off: usize,

	_phantom: PhantomData<T>,
}

impl<'a, T> ELFIterator<'a, T> {
	/// Creates a new iterator.
	///
	/// Arguments:
	/// - `table` is a slice to the table to iterate on.
	/// - `entsize` is the size of an entry in bytes.
	pub fn new(table: &'a [u8], entsize: usize) -> Self {
		Self {
			table,
			entsize,

			curr_off: 0,

			_phantom: PhantomData,
		}
	}
}

impl<'a, T: 'a> Iterator for ELFIterator<'a, T> {
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item> {
		if self.entsize == 0 || self.curr_off + self.entsize > self.table.len() {
			return None;
		}

		// FIXME: Not safety guarantee here. Ask for an empty unsafe trait on T to signal that the
		// structure has to be valid for every possible memory representations?
		let entry = unsafe {
			util::reinterpret::<T>(&self.table[self.curr_off..])
		}?;
		self.curr_off += self.entsize;
		Some(entry)
	}
}
