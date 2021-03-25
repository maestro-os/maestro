/// This module implements the String structure which wraps the `str` type.

use core::fmt::Debug;
use core::fmt;
use core::str;
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
	pub fn from(s: &str) -> Result::<Self, ()> {
		let mut v = Vec::new(); // TODO Reserve space
		for b in s.as_bytes() {
			v.push(*b)?;
		}

		Ok(Self {
			data: v,
		})
	}

	/// Returns a reference to the wrapped string.
	pub fn as_str(&self) -> &str {
		unsafe { // Call to unsafe function
			str::from_utf8_unchecked(self.data.as_slice())
		}
	}

	/// Returns the length of the String in characters count.
	pub fn len(&self) -> usize {
		self.as_str().len()
	}

	// TODO push
	// TODO pop

	/// Appends the string `other` to the current one.
	pub fn push_str(&mut self, other: &String) -> Result::<(), ()> {
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
	fn failable_clone(&self) -> Result::<Self, ()> {
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
