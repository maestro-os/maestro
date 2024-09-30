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

//! TODO doc

use core::ptr::NonNull;

/// A linked-list of buffers representing a packet being built.
///
/// This structure works without any memory allocations and relies entirely on lifetimes.
pub struct BuffList<'b> {
	/// The buffer.
	b: &'b [u8],

	/// The next buffer in the list.
	next: Option<NonNull<BuffList<'b>>>,
	/// The length of following buffers combined.
	next_len: usize,
}

impl<'b> From<&'b [u8]> for BuffList<'b> {
	fn from(b: &'b [u8]) -> Self {
		Self {
			b,

			next: None,
			next_len: 0,
		}
	}
}

impl<'b> BuffList<'b> {
	/// Returns the length of the buffer, plus following buffers.
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.b.len() + self.next_len
	}

	/// Pushes another buffer at the front of the current list.
	///
	/// The function returns the new head of the list (which is the given `front`).
	pub fn push_front<'o>(&mut self, mut front: BuffList<'o>) -> BuffList<'o>
	where
		'b: 'o,
	{
		front.next = NonNull::new(self);
		front.next_len = self.b.len() + self.next_len;

		front
	}
}
