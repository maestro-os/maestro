/// This module implements the String structure which wraps the `str` type.

use core::fmt::Debug;
use core::fmt;
use core::str;
use crate::errno::Errno;
use crate::util::FailableClone;
use crate::util::container::vec::Vec;

/// The String structure, which wraps the `str` primitive type.
pub struct String {
	/// A Vec containing the string's data.
	data: Vec::<u8>,
}

impl String {
	/// Creates a new instance of empty string.
	pub fn new() -> Self {
		Self {
			data: Vec::new(),
		}
	}

	/// Creates a new instance. If the string cannot be allocated, the function return Err.
	pub fn from(s: &str) -> Result<Self, Errno> {
		let bytes = s.as_bytes();
		let mut v = Vec::with_capacity(bytes.len())?;
		for b in bytes {
			v.push(*b)?;
		}

		Ok(Self {
			data: v,
		})
	}

	/// Returns a reference to the wrapped string.
	pub fn as_str(&self) -> &str {
		unsafe {
			str::from_utf8_unchecked(self.data.as_slice())
		}
	}

	/// Returns the length of the String in characters count.
	pub fn len(&self) -> usize {
		self.as_str().len()
	}

	/// Tells whether the string is empty.
	pub fn is_empty(&mut self) -> bool {
		self.data.is_empty()
	}

	// TODO Unit tests
	/// Appends the given char `ch` to the end of the string.
	pub fn push(&mut self, ch: char) -> Result<(), Errno> {
		match ch.len_utf8() {
			1 => self.data.push(ch as u8)?,
			_ => {
				let val = ch as u32;
				for i in 0..4 {
					if let Err(e) = self.data.push(((val >> (8 * i)) & 0xff) as _) {
						// TODO Clean
						for _ in 0..i {
							self.data.pop();
						}

						return Err(e);
					}
				}
			},
		}

		Ok(())
	}

	// TODO Unit tests
	/// Removes the last character from the string and returns it.
	/// If the string is empty, the function returns None.
	pub fn pop(&mut self) -> Option<char> {
		if self.is_empty() {
			None
		} else {
			// TODO
			None
		}
	}

	/// Appends the string `other` to the current one.
	pub fn push_str(&mut self, other: &String) -> Result<(), Errno> {
		let mut v = other.data.failable_clone()?;
		self.data.append(&mut v)
	}

	/// Turns the string into an empty string.
	pub fn clear(&mut self) {
		self.data.clear();
	}
}

impl Eq for String {}

impl PartialEq for String {
	fn eq(&self, other: &String) -> bool {
		self.data == other.data
	}
}

impl PartialEq<str> for String {
	fn eq(&self, other: &str) -> bool {
		if self.len() != other.len() {
			return false;
		}

		let bytes = other.as_bytes();
		for i in 0..bytes.len() {
			if self.data[i] != bytes[i] {
				return false;
			}
		}

		true
	}
}

impl PartialEq<&str> for String {
	fn eq(&self, other: &&str) -> bool {
		self == *other
	}
}

impl FailableClone for String {
	fn failable_clone(&self) -> Result<Self, Errno> {
		Ok(Self {
			data: self.data.failable_clone()?,
		})
	}
}

// TODO Iterators

impl Debug for String {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}
