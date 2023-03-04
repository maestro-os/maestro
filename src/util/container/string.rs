//! This module implements the String structure which wraps the `str` type.

use crate::errno::Errno;
use crate::util::container::vec::Vec;
use crate::util::FailableClone;
use core::borrow::Borrow;
use core::borrow::BorrowMut;
use core::fmt;
use core::fmt::Debug;
use core::fmt::Write;
use core::hash::Hash;
use core::hash::Hasher;
use core::ops::Add;
use core::ops::Deref;
use core::str;

/// The String structure, which wraps the `str` primitive type.
pub struct String {
	/// A Vec containing the string's data.
	data: Vec<u8>,
}

impl String {
	/// Creates a new instance of empty string.
	pub fn new() -> Self {
		Self {
			data: Vec::new(),
		}
	}

	/// Creates a new instance from the given byte slice.
	pub fn from(s: &[u8]) -> Result<Self, Errno> {
		let mut v = Vec::with_capacity(s.len())?;
		for b in s {
			v.push(*b)?;
		}

		Ok(Self {
			data: v,
		})
	}

	/// Returns a slice containing the bytes representation of the string.
	pub fn as_bytes(&self) -> &[u8] {
		self.data.as_slice()
	}

	/// Returns a mutable slice containing the bytes representation of the
	/// string.
	pub fn as_mut_bytes(&mut self) -> &mut [u8] {
		self.data.as_mut_slice()
	}

	/// Returns a reference to the wrapped string.
	/// If the string isn't a valid UTF-8 string, the function returns None.
	pub fn as_str(&self) -> Option<&str> {
		str::from_utf8(self.as_bytes()).ok()
	}

	/// Same as `as_str` except the function doesn't check the string is a
	/// correct UTF-8 sequence. If invalid, the behaviour is undefined.
	pub unsafe fn as_str_unchecked(&self) -> &str {
		str::from_utf8_unchecked(self.as_bytes())
	}

	/// Returns the length of the String in bytes.
	pub fn len(&self) -> usize {
		self.data.len()
	}

	/// Returns the length of the String in characters count.
	///
	/// If the string isn't a valid UTF-8 string, the function returns `None`.
	pub fn strlen(&self) -> Option<usize> {
		Some(self.as_str()?.len())
	}

	/// Tells whether the string is empty.
	pub fn is_empty(&self) -> bool {
		self.data.is_empty()
	}

	/// Appends the given byte `b` to the end of the string.
	pub fn push(&mut self, b: u8) -> Result<(), Errno> {
		self.data.push(b)
	}

	// TODO Adapt to non-UTF-8
	/// Appends the given char `ch` to the end of the string.
	pub fn push_char(&mut self, ch: char) -> Result<(), Errno> {
		match ch.len_utf8() {
			1 => self.data.push(ch as u8)?,

			_ => {
				let val = ch as u32;

				for i in 0..4 {
					if let Err(e) = self.data.push(((val >> (8 * i)) & 0xff) as _) {
						// Cancelling previous iterations
						for _ in 0..i {
							self.data.pop();
						}

						return Err(e);
					}
				}
			}
		}

		Ok(())
	}

	/// Removes the last byte from the string and returns it.
	///
	/// If the string is empty, the function returns `None`.
	pub fn pop(&mut self) -> Option<u8> {
		self.data.pop()
	}

	/// Appends the string `other` to the current one.
	pub fn append<S: AsRef<[u8]>>(&mut self, other: S) -> Result<(), Errno> {
		self.data.extend_from_slice(other.as_ref())
	}

	/// Turns the string into an empty string.
	pub fn clear(&mut self) {
		self.data.clear();
	}
}

impl Deref for String {
	type Target = [u8];

	fn deref(&self) -> &Self::Target {
		self.as_bytes()
	}
}

impl AsRef<[u8]> for String {
	fn as_ref(&self) -> &[u8] {
		self.as_bytes()
	}
}

impl Borrow<[u8]> for String {
	fn borrow(&self) -> &[u8] {
		self.as_bytes()
	}
}

impl BorrowMut<[u8]> for String {
	fn borrow_mut(&mut self) -> &mut [u8] {
		self.as_mut_bytes()
	}
}

impl Add for String {
	type Output = Result<Self, Errno>;

	fn add(mut self, other: Self) -> Self::Output {
		self.append(other)?;
		Ok(self)
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

impl Hash for String {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_bytes().hash(state);
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
		for b in self.as_bytes() {
			f.write_char(*b as char)?;
		}

		Ok(())
	}
}

impl fmt::Display for String {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for b in self.as_bytes() {
			f.write_char(*b as char)?;
		}

		Ok(())
	}
}

/// Writer used to turned a format into an allocated string.
pub struct StringWriter {
	/// The final string resulting from the formatting.
	pub final_str: Option<Result<String, Errno>>,
}

impl Write for StringWriter {
	fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
		match &mut self.final_str {
			Some(Ok(final_str)) => match final_str.append(s.as_bytes()) {
				Err(e) => self.final_str = Some(Err(e)),
				_ => {}
			},

			None => self.final_str = Some(String::from(s.as_bytes())),
			_ => {}
		}

		Ok(())
	}
}

/// This function must be used only through the `format` macro.
pub fn _format(args: fmt::Arguments) -> Result<String, Errno> {
	let mut w = StringWriter {
		final_str: None,
	};
	fmt::write(&mut w, args).unwrap();

	w.final_str.unwrap()
}

/// Builds an owned string from the given format string.
#[macro_export]
macro_rules! format {
	($($arg:tt)*) => {{
		$crate::util::container::string::_format(format_args!($($arg)*))
	}};
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn string_push0() {
		let mut s = String::new();
		assert_eq!(s.len(), 0);

		s.push(b'a').unwrap();
		assert_eq!(s.len(), 1);
		assert_eq!(s, "a");
	}

	#[test_case]
	fn string_push1() {
		let mut s = String::new();
		assert_eq!(s.len(), 0);

		for i in 0..10 {
			s.push(b'a').unwrap();
			assert_eq!(s.len(), i + 1);
		}
		assert_eq!(s, "aaaaaaaaaa");
	}
}
