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

use crate::{TryClone, TryToOwned};
use core::{borrow::Borrow, fmt, ops::Deref};

/// A clone-on-write smart pointer.
pub enum Cow<'a, B: 'a + ?Sized + TryToOwned> {
	/// This variant represents a borrowed value.
	Borrowed(&'a B),
	/// This variant represents a value after it has been copied.
	Owned(<B as TryToOwned>::Owned),
}

impl<'a, B: 'a + ?Sized + TryToOwned> Cow<'a, B> {
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
	pub fn into_owned(self) -> Result<<B as TryToOwned>::Owned, <B as TryToOwned>::Error> {
		match self {
			Self::Borrowed(r) => r.try_to_owned(),
			Self::Owned(v) => Ok(v),
		}
	}

	/// Returns a mutable reference to the owned data.
	pub fn to_mut(&mut self) -> Result<&mut <B as TryToOwned>::Owned, <B as TryToOwned>::Error> {
		if let Self::Borrowed(r) = self {
			*self = Self::Owned(r.try_to_owned()?);
		}
		match self {
			Self::Owned(v) => Ok(v),
			_ => unreachable!(),
		}
	}
}

impl<'a, B: 'a + ?Sized + TryToOwned> From<&'a B> for Cow<'a, B> {
	fn from(t: &'a B) -> Self {
		Self::Borrowed(t)
	}
}

impl<'a, B: 'a + ?Sized + TryToOwned> Borrow<B> for Cow<'a, B> {
	fn borrow(&self) -> &B {
		self.as_ref()
	}
}

impl<'a, B: 'a + ?Sized + TryToOwned> Deref for Cow<'a, B> {
	type Target = B;

	fn deref(&self) -> &B {
		self.as_ref()
	}
}

impl<'a, B: 'a + ?Sized + TryToOwned> AsRef<B> for Cow<'a, B> {
	fn as_ref(&self) -> &B {
		match self {
			Self::Borrowed(r) => r,
			Self::Owned(v) => v.borrow(),
		}
	}
}

impl<'a, B: 'a + ?Sized + TryToOwned, E> TryClone for Cow<'a, B>
where
	<B as TryToOwned>::Owned: TryClone<Error = E>,
{
	type Error = E;

	fn try_clone(&self) -> Result<Self, E> {
		Ok(match self {
			Self::Borrowed(r) => Self::Borrowed(*r),
			Self::Owned(v) => Self::Owned(v.try_clone()?),
		})
	}
}

impl<'a, B: 'a + ?Sized + TryToOwned + fmt::Display> fmt::Display for Cow<'a, B> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		fmt::Display::fmt(self.as_ref(), f)
	}
}

impl<'a, B: 'a + ?Sized + TryToOwned + fmt::Debug> fmt::Debug for Cow<'a, B> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
		fmt::Debug::fmt(self.as_ref(), f)
	}
}
