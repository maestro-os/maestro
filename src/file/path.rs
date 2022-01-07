//! This module handles files path.

use core::cmp::min;
use core::fmt;
use core::hash::Hash;
use core::ops::Add;
use core::ops::Index;
use core::ops::IndexMut;
use core::ops::Range;
use core::ops::RangeFrom;
use core::ops::RangeTo;
use crate::errno::Errno;
use crate::errno;
use crate::limits;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

// FIXME vuln: When reducing a path, must check if a components are a directory. If not:
// If `foo` is a directory and `bar` is a file, then `foo/bar/..` would reduce to `foo`

/// The character used as a path separator.
pub const PATH_SEPARATOR: char = '/';

/// A structure representing a path to a file.
#[derive(Hash)]
pub struct Path {
	/// Tells whether the path is absolute or relative.
	absolute: bool,
	/// An array containing the different parts of the path which are separated with `/`.
	parts: Vec<String>,
}

impl Path {
	/// Creates a new instance to the root directory.
	pub const fn root() -> Self {
		Self {
			absolute: true,
			parts: Vec::new(),
		}
	}

	/// Creates a new instance from string.
	/// `path` is the path.
	/// `user` tells whether the path was supplied by the user (to check the length and return an
	/// error if too long).
	pub fn from_str(path: &[u8], user: bool) -> Result<Self, Errno> {
		if user && path.len() + 1 >= limits::PATH_MAX {
			return Err(errno::ENAMETOOLONG);
		}

		let mut parts = Vec::new();
		for p in path.split(| c | *c == PATH_SEPARATOR as u8) {
			if p.len() + 1 >= limits::NAME_MAX {
				return Err(errno::ENAMETOOLONG);
			}

			if !p.is_empty() {
				parts.push(String::from(p)?)?;
			}
		}

		Ok(Self {
			absolute: path.len() > 0 && path[0] == PATH_SEPARATOR as u8,
			parts,
		})
	}

	/// Tells whether the path is absolute or not.
	pub fn is_absolute(&self) -> bool {
		self.absolute
	}

	/// Tells whether the path is empty.
	pub fn is_empty(&self) -> bool {
		self.parts.is_empty()
	}

	/// Returns the number of elements in the path, namely, the number of elements separated by
	/// `/`.
	pub fn get_elements_count(&self) -> usize {
		self.parts.len()
	}

	/// Converts the path into a String and returns it.
	pub fn as_string(&self) -> Result<String, Errno> {
		let mut s = String::new();
		if self.absolute {
			s.push(b'/')?;
		}

		for (i, p) in self.parts.iter().enumerate() {
			s.push_str(p)?;
			if i + 1 < self.parts.len() {
				s.push(b'/')?;
			}
		}

		Ok(s)
	}

	/// Pushes the given filename `filename` onto the path.
	pub fn push(&mut self, filename: String) -> Result<(), Errno> {
		if filename.len() + 1 >= limits::NAME_MAX {
			return Err(errno::ENAMETOOLONG);
		}

		self.parts.push(filename)
	}

	/// Pops the filename on top of the path.
	pub fn pop(&mut self) {
		self.parts.pop();
	}

	/// Tells whether the current path begins with the path `other`.
	pub fn begins_with(&self, other: &Self) -> bool {
		if self.absolute != other.absolute {
			return false;
		}

		let len = min(self.parts.len(), other.parts.len());
		for i in 0..len {
			if self.parts[i] != other.parts[i] {
				return false;
			}
		}

		true
	}

	/// Returns a subpath in the given range `range`.
	pub fn range(&self, range: Range<usize>) -> Result<Path, Errno> {
		Ok(Self {
			absolute: self.absolute,
			parts: self.parts.clone_range(range)?,
		})
	}

	/// Returns a subpath in the given range `range`.
	pub fn range_from(&self, range: RangeFrom<usize>) -> Result<Path, Errno> {
		Ok(Self {
			absolute: self.absolute,
			parts: self.parts.clone_range_from(range)?,
		})
	}

	/// Returns a subpath in the given range `range`.
	pub fn range_to(&self, range: RangeTo<usize>) -> Result<Path, Errno> {
		Ok(Self {
			absolute: self.absolute,
			parts: self.parts.clone_range_to(range)?,
		})
	}

	/// Reduces the path, removing all useless `.` and `..`.
	pub fn reduce(&mut self) -> Result<(), Errno> {
		let mut i = 0;
		while i < self.parts.len() {
			let part = &self.parts[i];

			if part == "." || (part == ".." && self.absolute && i == 0) {
				self.parts.remove(i);
			} else if part == ".." && i > 0 && self.parts[i - 1] != ".." {
				self.parts.remove(i);
				self.parts.remove(i - 1);
				i -= 1;
			} else {
				i += 1;
			}
		}

		if !self.absolute && self.parts.is_empty() {
			self.parts.push(String::from(b".")?)?;
		}

		Ok(())
	}

	/// Concats the current path with another path `other` to create a new path. The path is not
	/// automaticaly reduced.
	/// If the `other` path is absolute, the resulting path exactly equals `other`.
	pub fn concat(&self, other: &Self) -> Result<Self, Errno> {
		if other.is_absolute() {
			other.failable_clone()
		} else {
			let mut self_parts = self.parts.failable_clone()?;
			let mut other_parts = other.parts.failable_clone()?;
			self_parts.append(&mut other_parts)?;

			Ok(Self {
				absolute: self.absolute,
				parts: self_parts,
			})
		}
	}
}

impl Add for Path {
	type Output = Result<Self, Errno>;

	fn add(self, other: Self) -> Self::Output {
		self.concat(&other)
	}
}

impl FailableClone for Path {
	fn failable_clone(&self) -> Result<Self, Errno> {
		Ok(Self {
			absolute: self.absolute,
			parts: self.parts.failable_clone()?,
		})
	}
}

impl Index<usize> for Path {
	type Output = String;

	#[inline]
	fn index(&self, index: usize) -> &Self::Output {
		&self.parts[index]
	}
}

impl IndexMut<usize> for Path {
	#[inline]
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.parts[index]
	}
}

impl Eq for Path {}

impl PartialEq for Path {
	fn eq(&self, other: &Self) -> bool {
		for i in 0..self.parts.len() {
			if self.parts[i] != other.parts[i] {
				return false;
			}
		}

		true
	}
}

// TODO Iterator

impl fmt::Display for Path {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		if self.is_absolute() {
			write!(f, "/")?;
		}

		for i in 0..self.get_elements_count() {
			write!(f, "{}", self[i])?;
			if i + 1 < self.get_elements_count() {
				write!(f, "/")?;
			}
		}

		Ok(())
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn path_absolute0() {
		assert!(Path::from_str(b"/", false).unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute1() {
		assert!(Path::from_str(b"/.", false).unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute2() {
		assert!(!Path::from_str(b".", false).unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute3() {
		assert!(!Path::from_str(b"..", false).unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute4() {
		assert!(!Path::from_str(b"./", false).unwrap().is_absolute());
	}

	#[test_case]
	fn path_reduce0() {
		let mut path = Path::from_str(b"/.", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "/");
	}

	#[test_case]
	fn path_reduce1() {
		let mut path = Path::from_str(b"/..", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "/");
	}

	#[test_case]
	fn path_reduce2() {
		let mut path = Path::from_str(b"./", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), ".");
	}

	#[test_case]
	fn path_reduce3() {
		let mut path = Path::from_str(b"../", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "..");
	}

	#[test_case]
	fn path_reduce4() {
		let mut path = Path::from_str(b"../bleh", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "../bleh");
	}

	#[test_case]
	fn path_reduce5() {
		let mut path = Path::from_str(b"../bleh/..", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "..");
	}

	#[test_case]
	fn path_reduce6() {
		let mut path = Path::from_str(b"../bleh/../bluh", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "../bluh");
	}

	#[test_case]
	fn path_reduce7() {
		let mut path = Path::from_str(b"/bleh/../bluh", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "/bluh");
	}

	#[test_case]
	fn path_reduce8() {
		let mut path = Path::from_str(b"/bleh/../../bluh", false).unwrap();
		path.reduce().unwrap();
		assert_eq!(path.as_string().unwrap(), "/bluh");
	}

	// TODO test concat
}
