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

//! This module implements a generic iterator to be used on tables in ELF files.

use crate::util;
use core::marker::PhantomData;

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

		// FIXME: No safety guarantee here. Ask for an empty unsafe trait on T to signal that the
		// structure has to be valid for every possible memory representations?
		let entry = unsafe { util::reinterpret::<T>(&self.table[self.curr_off..]) }?;
		self.curr_off += self.entsize;
		Some(entry)
	}

	fn count(self) -> usize {
		self.table.len() / self.entsize
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.table.len() / self.entsize;
		(len, Some(len))
	}

	fn nth(&mut self, n: usize) -> Option<Self::Item> {
		self.curr_off += n * self.entsize;
		self.next()
	}
}
