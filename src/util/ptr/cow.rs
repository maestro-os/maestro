//! This module implements Copy-On-Write (COW) pointers.

use crate::errno::Errno;
use crate::util::FailableClone;
use core::borrow::Borrow;
use core::fmt;

/// Structure implementing a copy-on-write smart pointer.
pub enum Cow<'a, T: 'a + FailableClone> {
	/// This variant represents a borrowed value.
	Borrowed(&'a T),
	/// This variant represents a value after it has been copied.
	Owned(T),
}

impl<'a, T: 'a + FailableClone> Cow<'a, T> {
	/// Tells whether the object is a borrowed value.
	pub fn is_borrowed(&self) -> bool {
		match self {
			Self::Borrowed(_) => true,
			Self::Owned(_) => false,
		}
	}

	/// Tells whether the object is an owned value.
	pub fn is_owned(&self) -> bool {
		!self.is_borrowed()
	}

	/// Turns the wrapped value into an owned version. This function clones the
	/// value if necessary. On fail, the function returns an error.
	pub fn into_owned(self) -> Result<T, Errno> {
		match self {
			Self::Borrowed(r) => r.failable_clone(),
			Self::Owned(v) => Ok(v),
		}
	}

	/// Returns a mutable reference to the owned data.
	pub fn to_mut(&mut self) -> Result<&mut T, Errno> {
		match self {
			Self::Borrowed(r) => *self = Self::Owned(r.failable_clone()?),
			_ => {}
		}

		match self {
			Self::Owned(v) => Ok(v),
			_ => unreachable!(),
		}
	}
}

impl<'a, T: 'a + FailableClone> From<T> for Cow<'a, T> {
	fn from(t: T) -> Self {
		Self::Owned(t)
	}
}

impl<'a, T: 'a + FailableClone> From<&'a T> for Cow<'a, T> {
	fn from(t: &'a T) -> Self {
		Self::Borrowed(t)
	}
}

impl<'a, T: 'a + FailableClone> Borrow<T> for Cow<'a, T> {
	fn borrow(&self) -> &T {
		self.as_ref()
	}
}

impl<'a, T: 'a + FailableClone> AsRef<T> for Cow<'a, T> {
	fn as_ref(&self) -> &T {
		match self {
			Self::Borrowed(r) => r,
			Self::Owned(v) => &v,
		}
	}
}

impl<'a, T: 'a + FailableClone + fmt::Display> fmt::Display for Cow<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		self.as_ref().fmt(f)
	}
}

// TODO Implement comparison and arithmetic
