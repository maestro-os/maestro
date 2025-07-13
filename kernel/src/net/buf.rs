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

//! Buffer list, to avoid allocations in network code.

use core::ptr::NonNull;

/// A linked-list of buffers representing a packet being built.
pub struct BufList<'b> {
	/// The buffer.
	pub data: &'b [u8],

	/// The next buffer in the list.
	next: Option<NonNull<BufList<'b>>>,
	/// The length of following buffers combined.
	next_len: usize,
}

impl<'b> From<&'b [u8]> for BufList<'b> {
	fn from(b: &'b [u8]) -> Self {
		Self {
			data: b,

			next: None,
			next_len: 0,
		}
	}
}

impl<'b> BufList<'b> {
	/// Returns the length of the buffer, plus following buffers.
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.data.len() + self.next_len
	}

	/// Pushes another buffer at the front of the current list.
	///
	/// The function returns the new head of the list (which is the given `front`).
	pub fn push_front<'o>(&mut self, mut front: BufList<'o>) -> BufList<'o>
	where
		'b: 'o,
	{
		front.next = NonNull::new(self);
		front.next_len = self.data.len() + self.next_len;
		front
	}

	/// Returns the next buffer.
	pub fn next(&self) -> Option<&BufList<'b>> {
		unsafe { self.next.map(|n| n.as_ref()) }
	}
}
