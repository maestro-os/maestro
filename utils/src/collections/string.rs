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

//! Owned version of `[u8]` (in standard Rust, this would be an owned version of `str` instead).

use crate::{
	AllocError, TryClone, TryToOwned,
	collections::{
		path::{Path, PathBuf},
		vec::Vec,
	},
	errno::{AllocResult, CollectResult},
};
use core::{
	borrow::{Borrow, BorrowMut},
	fmt,
	fmt::{Arguments, Debug, Write},
	hash::{Hash, Hasher},
	mem,
	ops::{Add, Deref},
	str,
};

/// The String structure, which wraps the `str` primitive type.
#[derive(Default)]
pub struct String {
	/// A Vec containing the string's data.
	data: Vec<u8>,
}

impl String {
	/// Creates a new instance of empty string.
	pub const fn new() -> Self {
		Self {
			data: Vec::new(),
		}
	}

	/// Creates a new instance with the given capacity in bytes.
	pub fn with_capacity(capacity: usize) -> AllocResult<Self> {
		Ok(Self {
			data: Vec::with_capacity(capacity)?,
		})
	}

	/// Returns the capacity of the inner buffer, in bytes.
	pub fn capacity(&self) -> usize {
		self.data.capacity()
	}

	/// Returns a slice containing the bytes representation of the string.
	#[inline]
	pub fn as_bytes(&self) -> &[u8] {
		self.data.as_slice()
	}

	/// Returns a mutable slice containing the bytes representation of the
	/// string.
	#[inline]
	pub fn as_mut_bytes(&mut self) -> &mut [u8] {
		self.data.as_mut_slice()
	}

	/// Returns a reference to the wrapped string.
	///
	/// If the string isn't a valid UTF-8 string, the function returns `None`.
	#[inline]
	pub fn as_str(&self) -> Option<&str> {
		str::from_utf8(self.as_bytes()).ok()
	}

	/// Same as `as_str` except the function doesn't check the string is a
	/// correct UTF-8 sequence.
	///
	/// # Safety
	///
	/// If the string is not a valid in UTF-8 encoding, the behavior is undefined.
	#[inline]
	pub unsafe fn as_str_unchecked(&self) -> &str {
		str::from_utf8_unchecked(self.as_bytes())
	}

	/// Returns the length of the String in bytes.
	#[inline]
	pub fn len(&self) -> usize {
		self.data.len()
	}

	/// Returns the length of the String in characters count.
	///
	/// If the string isn't a valid UTF-8 string, the function returns `None`.
	#[inline]
	pub fn strlen(&self) -> Option<usize> {
		Some(self.as_str()?.len())
	}

	/// Tells whether the string is empty.
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.data.is_empty()
	}

	/// Appends the given byte `b` to the end of the string.
	#[inline]
	pub fn push(&mut self, b: u8) -> AllocResult<()> {
		self.data.push(b)
	}

	/// Appends the given char `ch` to the end of the string.
	pub fn push_char(&mut self, ch: char) -> AllocResult<()> {
		if ch.len_utf8() == 1 {
			return self.data.push(ch as u8);
		}

		let val = ch as u32;
		for i in 0..4 {
			let b = ((val >> (8 * i)) & 0xff) as u8;
			if let Err(e) = self.data.push(b) {
				// Cancelling previous iterations
				self.data.truncate(self.data.len() - i);
				return Err(e);
			}
		}

		Ok(())
	}

	/// Removes the last byte from the string and returns it.
	///
	/// If the string is empty, the function returns `None`.
	#[inline]
	pub fn pop(&mut self) -> Option<u8> {
		self.data.pop()
	}

	/// Appends the string `other` to the current.
	#[inline]
	pub fn push_str<S: AsRef<[u8]>>(&mut self, other: S) -> AllocResult<()> {
		self.data.extend_from_slice(other.as_ref())
	}

	/// Turns the string into an empty string.
	#[inline]
	pub fn clear(&mut self) {
		self.data.clear();
	}
}

impl From<Vec<u8>> for String {
	fn from(data: Vec<u8>) -> Self {
		Self {
			data,
		}
	}
}

impl From<PathBuf> for String {
	fn from(path: PathBuf) -> Self {
		path.0
	}
}

impl TryFrom<&[u8]> for String {
	type Error = AllocError;

	fn try_from(s: &[u8]) -> Result<Self, Self::Error> {
		Ok(Self {
			data: Vec::try_from(s)?,
		})
	}
}

impl TryFrom<&Path> for String {
	type Error = AllocError;

	fn try_from(path: &Path) -> Result<Self, Self::Error> {
		Self::try_from(path.as_bytes())
	}
}

impl<const N: usize> TryFrom<&[u8; N]> for String {
	type Error = AllocError;

	fn try_from(s: &[u8; N]) -> Result<Self, Self::Error> {
		Self::try_from(s.as_slice())
	}
}

impl TryToOwned for [u8] {
	type Owned = String;

	fn try_to_owned(&self) -> Result<Self::Owned, Self::Error> {
		String::try_from(self)
	}
}

impl TryFrom<&str> for String {
	type Error = AllocError;

	fn try_from(s: &str) -> Result<Self, Self::Error> {
		Self::try_from(s.as_bytes())
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
	type Output = Result<Self, AllocError>;

	fn add(mut self, other: Self) -> Self::Output {
		self.push_str(other)?;
		Ok(self)
	}
}

impl Eq for String {}

impl PartialEq for String {
	fn eq(&self, other: &String) -> bool {
		self.data == other.data
	}
}

impl PartialEq<[u8]> for String {
	fn eq(&self, other: &[u8]) -> bool {
		if self.len() != other.len() {
			return false;
		}

		for (a, b) in self.data.iter().zip(other.iter()) {
			if a != b {
				return false;
			}
		}

		true
	}
}

impl PartialEq<str> for String {
	fn eq(&self, other: &str) -> bool {
		self.eq(other.as_bytes())
	}
}

impl PartialEq<&str> for String {
	fn eq(&self, other: &&str) -> bool {
		self.eq(other.as_bytes())
	}
}

impl Hash for String {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_bytes().hash(state);
	}
}

impl TryClone for String {
	fn try_clone(&self) -> AllocResult<Self> {
		Ok(Self {
			data: self.data.try_clone()?,
		})
	}
}

impl FromIterator<u8> for CollectResult<String> {
	fn from_iter<T: IntoIterator<Item = u8>>(iter: T) -> Self {
		Self(
			CollectResult::<Vec<u8>>::from_iter(iter)
				.0
				.map(String::from),
		)
	}
}

impl<'c> FromIterator<&'c u8> for CollectResult<String> {
	fn from_iter<T: IntoIterator<Item = &'c u8>>(iter: T) -> Self {
		Self(
			CollectResult::<Vec<u8>>::from_iter(iter)
				.0
				.map(String::from),
		)
	}
}

// TODO Iterators

impl fmt::Display for String {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for b in self.as_bytes() {
			f.write_char(*b as char)?;
		}
		Ok(())
	}
}

impl Debug for String {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "\"{self}\"")
	}
}

/// Fallible writer for [`String`].
pub struct StringWriter(pub AllocResult<String>);

impl Write for StringWriter {
	fn write_str(&mut self, s: &str) -> Result<(), fmt::Error> {
		let buf = mem::replace(&mut self.0, Ok(String::default()));
		self.0 = buf.and_then(|mut buf| {
			buf.push_str(s)?;
			Ok(buf)
		});
		Ok(())
	}
}

/// Formats a string from the given arguments.
///
/// On memory allocation failure, the function returns an error.
pub fn format_impl(args: Arguments<'_>) -> AllocResult<String> {
	let buf = String::with_capacity(args.estimated_capacity());
	let mut w = StringWriter(buf);
	fmt::write(&mut w, args).expect("a formatting trait implementation returned an error");
	w.0
}

/// Formats a [`String`] from the given arguments.
///
/// On memory allocation failure, the macro returns an error.
#[macro_export]
macro_rules! format {
	($($arg:tt)*) => {{
		$crate::collections::string::format_impl(format_args!($($arg)*))
	}};
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn string_push0() {
		let mut s = String::new();
		assert_eq!(s.len(), 0);

		s.push(b'a').unwrap();
		assert_eq!(s.len(), 1);
		assert_eq!(s, "a");
	}

	#[test]
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
