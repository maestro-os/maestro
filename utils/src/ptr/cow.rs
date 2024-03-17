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

//! This module implements Copy-On-Write (COW) pointers.

use crate::TryClone;
use core::{borrow::Borrow, fmt};

/// Structure implementing a copy-on-write smart pointer.
pub enum Cow<'a, T: 'a + TryClone> {
	/// This variant represents a borrowed value.
	Borrowed(&'a T),
	/// This variant represents a value after it has been copied.
	Owned(T),
}

impl<'a, T: 'a + TryClone<Error = E>, E> Cow<'a, T> {
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

	/// Turns the wrapped value into an owned version.
	///
	/// This function clones the value if necessary.
	///
	/// On fail, the function returns an error.
	pub fn into_owned(self) -> Result<T, E> {
		match self {
			Self::Borrowed(r) => r.try_clone(),
			Self::Owned(v) => Ok(v),
		}
	}

	/// Returns a mutable reference to the owned data.
	pub fn to_mut(&mut self) -> Result<&mut T, E> {
		if let Self::Borrowed(r) = self {
			*self = Self::Owned(r.try_clone()?);
		}

		match self {
			Self::Owned(v) => Ok(v),
			_ => unreachable!(),
		}
	}
}

impl<'a, T: 'a + TryClone> From<T> for Cow<'a, T> {
	fn from(t: T) -> Self {
		Self::Owned(t)
	}
}

impl<'a, T: 'a + TryClone> From<&'a T> for Cow<'a, T> {
	fn from(t: &'a T) -> Self {
		Self::Borrowed(t)
	}
}

impl<'a, T: 'a + TryClone> Borrow<T> for Cow<'a, T> {
	fn borrow(&self) -> &T {
		self.as_ref()
	}
}

impl<'a, T: 'a + TryClone> AsRef<T> for Cow<'a, T> {
	fn as_ref(&self) -> &T {
		match self {
			Self::Borrowed(r) => r,
			Self::Owned(v) => v,
		}
	}
}

impl<'a, T: 'a + TryClone + fmt::Display> fmt::Display for Cow<'a, T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		self.as_ref().fmt(f)
	}
}

// TODO Implement comparison and arithmetic
