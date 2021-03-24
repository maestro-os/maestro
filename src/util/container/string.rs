/// This module implements the String structure which wraps the `str` type.

use crate::util::FailableClone;
use crate::util::boxed::Box;

/// The String structure, which wraps the `str` primitive type.
pub struct String {
	/// A box containing the string's data.
	data: Box::<str>,
}

impl String {
	/// Creates a new instance. If the string cannot be allocated, the function return Err.
	pub fn from(_s: &str) -> Result::<Self, ()> {
		// TODO
		Err(())
	}

	// TODO push
	// TODO pop
	// TODO clear
}

impl Eq for String {}

impl PartialEq for String {
	fn eq(&self, other: &String) -> bool {
		*self.data == *other.data
	}
}

impl PartialEq<str> for String {
	fn eq(&self, other: &str) -> bool {
		*self.data == *other
	}
}

impl PartialEq<&str> for String {
	fn eq(&self, other: &&str) -> bool {
		*self.data == **other
	}
}

impl FailableClone for String {
	fn failable_clone(&self) -> Result::<Self, ()> {
		// TODO
		Err(())
	}
}
