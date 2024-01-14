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
