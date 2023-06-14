//! The Open Systems Interconnection (OSI) model defines the architecure of a network stack.

use crate::errno::Errno;
use crate::util::container::vec::Vec;
use core::ptr;
use core::ptr::NonNull;

/// A linked-list of buffers representing a packet being built.
pub struct BuffList<'b> {
	/// The buffer.
	b: &'b [u8],

	/// The next buffer in the list.
	next: Option<NonNull<BuffList<'b>>>,
	/// The length of following buffers.
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
	pub fn len(&self) -> usize {
		self.b.len() + self.next_len
	}

	/// Pushes another buffer at the front of the list.
	pub fn push_front<'o>(&mut self, mut other: BuffList<'o>) -> BuffList<'o>
	where
		'b: 'o,
	{
		other.next = NonNull::new(self);
		other.next_len = self.b.len() + self.next_len;

		other
	}

	/// Collects all buffers into one.
	pub fn collect(&self) -> Result<Vec<u8>, Errno> {
		let len = self.len();
		let mut final_buff = crate::vec![0; len]?;

		let mut node = NonNull::new(self as *const _ as *mut Self);
		let mut i = 0;
		while let Some(mut curr) = node {
			let curr = unsafe { curr.as_mut() };
			let buf = curr.b;
			unsafe {
				ptr::copy_nonoverlapping(buf.as_ptr(), &mut final_buff[i], buf.len());
			}

			node = curr.next;
			i += buf.len();
		}

		Ok(final_buff)
	}
}

/// An OSI layer.
///
/// A layer stack acts as a pipeline, passing data from one layer to the other.
pub trait Layer {
	// TODO receive

	/// Transmits data in the given buffer.
	///
	/// Arguments:
	/// - `buff` is the list of buffer which composes the packet being built.
	/// - `next` is the function called to pass the buffers list to the next layer.
	fn transmit<'c, F>(&self, buff: BuffList<'c>, next: F) -> Result<(), Errno>
	where
		Self: Sized,
		F: Fn(BuffList<'c>) -> Result<(), Errno>;
}
