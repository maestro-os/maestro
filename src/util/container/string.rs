//! This module implements the String structure which wraps the `str` type.

use core::fmt::Debug;
use core::fmt;
use core::hash::Hash;
use core::hash::Hasher;
use core::str;
use crate::errno::Errno;
use crate::util::FailableClone;
use crate::util::container::vec::Vec;
use crate::util::math;

/// Returns the number of characters required to represent the given number `n` as a String.
fn get_number_len(mut n: i64, base: u8) -> usize {
	if n == 0 {
		1
	} else {
		let mut len = 0;

		if n < 0 {
			len += 1;
		}

		while n != 0 {
			len += 1;
			n /= base as i64;
		}

		len
	}
}

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

	// TODO Support other bases than only 10?
	// TODO Use a generic type?
	// TODO Optimize
	/// Creates a new instance filled with the string representation of a given number `n`.
	pub fn from_number(n: i64) -> Result<Self, Errno> {
		let len = get_number_len(n, 10);
		debug_assert!(len > 0);
		let mut v = Vec::with_capacity(len)?;

		let mut l = len;
		if n < 0 {
			v.push(b'-')?;
			l -= 1;
		}
		for i in (0..l).rev() {
			let b = {
				if i == 0 {
					(n % 10).abs() as u8
				} else {
					let shift = math::pow(10, i) as i64;
					(n / shift % 10).abs() as u8
				}
			};

			v.push(b'0' + b)?;
		}
		debug_assert_eq!(v.len(), len);

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

	/// Returns a slice containing the bytes representation of the string.
	pub fn as_bytes(&self) -> &[u8] {
		self.as_str().as_bytes()
	}

	/// Returns the length of the String in characters count.
	pub fn len(&self) -> usize {
		self.as_str().len()
	}

	/// Tells whether the string is empty.
	pub fn is_empty(&mut self) -> bool {
		self.data.is_empty()
	}

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

impl Hash for String {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.as_str().hash(state);
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

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn string_from_number0() {
		assert_eq!(String::from_number(0).unwrap(), "0");
		assert_eq!(String::from_number(1).unwrap(), "1");
		assert_eq!(String::from_number(2).unwrap(), "2");
		assert_eq!(String::from_number(3).unwrap(), "3");
		assert_eq!(String::from_number(4).unwrap(), "4");
		assert_eq!(String::from_number(5).unwrap(), "5");
		assert_eq!(String::from_number(6).unwrap(), "6");
		assert_eq!(String::from_number(7).unwrap(), "7");
		assert_eq!(String::from_number(8).unwrap(), "8");
		assert_eq!(String::from_number(9).unwrap(), "9");
	}

	#[test_case]
	fn string_from_number1() {
		assert_eq!(String::from_number(10).unwrap(), "10");
		assert_eq!(String::from_number(11).unwrap(), "11");
		assert_eq!(String::from_number(12).unwrap(), "12");
		assert_eq!(String::from_number(13).unwrap(), "13");
		assert_eq!(String::from_number(14).unwrap(), "14");
		assert_eq!(String::from_number(15).unwrap(), "15");
		assert_eq!(String::from_number(16).unwrap(), "16");
		assert_eq!(String::from_number(17).unwrap(), "17");
		assert_eq!(String::from_number(18).unwrap(), "18");
		assert_eq!(String::from_number(19).unwrap(), "19");
	}

	#[test_case]
	fn string_from_number2() {
		assert_eq!(String::from_number(-1).unwrap(), "-1");
		assert_eq!(String::from_number(-2).unwrap(), "-2");
		assert_eq!(String::from_number(-3).unwrap(), "-3");
		assert_eq!(String::from_number(-4).unwrap(), "-4");
		assert_eq!(String::from_number(-5).unwrap(), "-5");
		assert_eq!(String::from_number(-6).unwrap(), "-6");
		assert_eq!(String::from_number(-7).unwrap(), "-7");
		assert_eq!(String::from_number(-8).unwrap(), "-8");
		assert_eq!(String::from_number(-9).unwrap(), "-9");
	}

	#[test_case]
	fn string_from_number3() {
		assert_eq!(String::from_number(-10).unwrap(), "-10");
		assert_eq!(String::from_number(-11).unwrap(), "-11");
		assert_eq!(String::from_number(-12).unwrap(), "-12");
		assert_eq!(String::from_number(-13).unwrap(), "-13");
		assert_eq!(String::from_number(-14).unwrap(), "-14");
		assert_eq!(String::from_number(-15).unwrap(), "-15");
		assert_eq!(String::from_number(-16).unwrap(), "-16");
		assert_eq!(String::from_number(-17).unwrap(), "-17");
		assert_eq!(String::from_number(-18).unwrap(), "-18");
		assert_eq!(String::from_number(-19).unwrap(), "-19");
	}

	#[test_case]
	fn string_from_number4() {
		assert_eq!(String::from_number(100).unwrap(), "100");
		assert_eq!(String::from_number(101).unwrap(), "101");
		assert_eq!(String::from_number(102).unwrap(), "102");
		assert_eq!(String::from_number(103).unwrap(), "103");
		assert_eq!(String::from_number(104).unwrap(), "104");
		assert_eq!(String::from_number(105).unwrap(), "105");
		assert_eq!(String::from_number(106).unwrap(), "106");
		assert_eq!(String::from_number(107).unwrap(), "107");
		assert_eq!(String::from_number(108).unwrap(), "108");
		assert_eq!(String::from_number(109).unwrap(), "109");
	}

	#[test_case]
	fn string_from_number5() {
		assert_eq!(String::from_number(1000).unwrap(), "1000");
		assert_eq!(String::from_number(1001).unwrap(), "1001");
		assert_eq!(String::from_number(1002).unwrap(), "1002");
		assert_eq!(String::from_number(1003).unwrap(), "1003");
		assert_eq!(String::from_number(1004).unwrap(), "1004");
		assert_eq!(String::from_number(1005).unwrap(), "1005");
		assert_eq!(String::from_number(1006).unwrap(), "1006");
		assert_eq!(String::from_number(1007).unwrap(), "1007");
		assert_eq!(String::from_number(1008).unwrap(), "1008");
		assert_eq!(String::from_number(1009).unwrap(), "1009");
	}

	#[test_case]
	fn string_from_number6() {
		assert_eq!(String::from_number(-101).unwrap(), "-101");
		assert_eq!(String::from_number(-102).unwrap(), "-102");
		assert_eq!(String::from_number(-103).unwrap(), "-103");
		assert_eq!(String::from_number(-104).unwrap(), "-104");
		assert_eq!(String::from_number(-105).unwrap(), "-105");
		assert_eq!(String::from_number(-106).unwrap(), "-106");
		assert_eq!(String::from_number(-107).unwrap(), "-107");
		assert_eq!(String::from_number(-108).unwrap(), "-108");
		assert_eq!(String::from_number(-109).unwrap(), "-109");
	}

	#[test_case]
	fn string_from_number7() {
		assert_eq!(String::from_number(-1000).unwrap(), "-1000");
		assert_eq!(String::from_number(-1001).unwrap(), "-1001");
		assert_eq!(String::from_number(-1002).unwrap(), "-1002");
		assert_eq!(String::from_number(-1003).unwrap(), "-1003");
		assert_eq!(String::from_number(-1004).unwrap(), "-1004");
		assert_eq!(String::from_number(-1005).unwrap(), "-1005");
		assert_eq!(String::from_number(-1006).unwrap(), "-1006");
		assert_eq!(String::from_number(-1007).unwrap(), "-1007");
		assert_eq!(String::from_number(-1008).unwrap(), "-1008");
		assert_eq!(String::from_number(-1009).unwrap(), "-1009");
	}

	// TODO Test min and max values

	#[test_case]
	fn string_push0() {
		let mut s = String::new();
		assert_eq!(s.len(), 0);

		s.push('a').unwrap();
		assert_eq!(s.len(), 1);
		assert_eq!(s, "a");
	}

	#[test_case]
	fn string_push1() {
		let mut s = String::new();
		assert_eq!(s.len(), 0);

		for i in 0..10 {
			s.push('a').unwrap();
			assert_eq!(s.len(), i + 1);
		}
		assert_eq!(s, "aaaaaaaaaa");
	}
}
