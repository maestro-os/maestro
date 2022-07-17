//! This module implements an accumulator, which concatenates strings iteratively to dynamicaly
//! generate the content of a file.

use core::cmp::min;
use crate::errno::Errno;
use crate::util::container::string::String;
use crate::util;

/// Structure representing an accumulator.
/// Each call to `next` returns the next byte.
pub struct Accumulator<F: FnMut() -> Option<Result<String, Errno>>> {
	/// The function to call to get the next element.
	f: F,
}

impl<F: FnMut() -> Option<Result<String, Errno>>> Accumulator<F> {
	/// Creates a new accumulator with the given closure as data source.
	pub fn new(f: F) -> Self {
		Self {
			f,
		}
	}

	/// Extracts the string at the given offset `offset`.
	/// `buff` is the slice to be filled with the string extract.
	/// On success, the function returns the number of bytes written.
	pub fn extract(mut self, offset: usize, buff: &mut [u8]) -> Result<usize, Errno> {
		let mut remains: Option<(usize, String)> = None;

		// Skipping `offset` bytes
		let mut i = 0;
		while i < offset {
			match (self.f)() {
				Some(s) => {
					let s = s?;

					let step = min(offset - i, s.len());
					remains = Some((step, s));

					i += step;
				},

				None => break,
			}
		}

		// Copying strings to slice
		let mut i = 0;
		if let Some((off, s)) = remains {
			let copy_len = min(buff.len() - i, s.len() - off);
			util::slice_copy(s.as_bytes(), &mut buff[i..(i + copy_len)]);

			i += copy_len;
		}
		while i < buff.len() {
			match (self.f)() {
				Some(s) => {
					let s = s?;

					let copy_len = min(buff.len() - i, s.len());
					util::slice_copy(s.as_bytes(), &mut buff[i..(i + copy_len)]);

					i += copy_len;
				},

				None => break,
			}
		}

		Ok(i)
	}
}
