/// This module handles files path.

use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;

/// A structure representing a path to a file.
pub struct Path {
	/// Tells whether the path is absolute or relative.
	absolute: bool,
	/// An array containing the different parts of the path which are separated with `/`.
	parts: Vec::<String>,
}

impl Path {
	/// Creates a new instance from string.
	pub fn from_string(_path: &str) -> Self {
		// TODO
		Self {
			absolute: false,
			parts: Vec::new(),
		}
	}

	/// Tells whether the path is absolute or not.
	pub fn is_absolute(&self) -> bool {
		self.absolute
	}

	// TODO to_string

	/// Reduces the path, removing all useless `.` and `..`.
	pub fn reduce(&mut self) {
		let mut i = 0;
		while i < self.parts.len() {
			let part = &self.parts[i];
			if part == "." {
				self.parts.remove(i);
			} else if part == ".." && self.absolute && i == 0 {
				self.parts.remove(i);
			} else if part == ".." && i > 0 && self.parts[i - 1] != ".." {
				self.parts.remove(i);
				self.parts.remove(i - 1);
				i -= 1;
			} else {
				i += 1;
			}
		}
	}

	/// Concats the current path with another path `other` to create a new path.
	pub fn concat(&self, other: &Self) -> Result::<Self, ()> {
		let mut self_parts = self.parts.failable_clone()?;
		let mut other_parts = other.parts.failable_clone()?;
		self_parts.append(&mut other_parts)?;
		Ok(Self {
			absolute: self.absolute,
			parts: self_parts,
		})
	}
}

impl FailableClone for Path {
	fn failable_clone(&self) -> Result::<Self, ()> {
		// TODO
		Err(())
	}
}

// TODO Unit tests
