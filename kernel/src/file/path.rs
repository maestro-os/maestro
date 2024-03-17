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

//! This module implements structure to represent file paths.

use crate::limits;
use core::{
	alloc::AllocError, borrow::Borrow, fmt, fmt::Formatter, hash::Hash, iter::FusedIterator,
	ops::Deref,
};
use utils::{
	collections::string::String,
	errno,
	errno::{AllocResult, CollectResult, EResult, Errno},
	DisplayableStr, TryClone,
};

/// The character used as a path separator.
pub const PATH_SEPARATOR: u8 = b'/';

/// Owned file path.
#[derive(Default, Eq, Hash, PartialEq)]
pub struct PathBuf(String);

impl PathBuf {
	/// Creates a new path to root.
	pub fn root() -> Self {
		Self(String::default())
	}

	/// Creates a `PathBuf` without checking the length of the given [`String`].
	pub fn new_unchecked(s: String) -> Self {
		Self(s)
	}
}

impl TryFrom<String> for PathBuf {
	type Error = Errno;

	/// Creates a new instance from the given string.
	///
	/// If the total length of the path is longer than [`limits::PATH_MAX`], the function returns
	/// an error ([`errno::ENAMETOOLONG`]).
	fn try_from(s: String) -> EResult<Self> {
		if s.len() > limits::PATH_MAX {
			return Err(errno!(ENAMETOOLONG));
		}
		Ok(Self(s))
	}
}

impl TryClone for PathBuf {
	type Error = AllocError;

	fn try_clone(&self) -> AllocResult<Self> {
		Ok(Self(self.0.try_clone()?))
	}
}

impl TryFrom<&Path> for PathBuf {
	type Error = AllocError;

	fn try_from(s: &Path) -> AllocResult<Self> {
		Ok(Self(String::try_from(&s.0)?))
	}
}

impl<const N: usize> TryFrom<&[u8; N]> for PathBuf {
	type Error = Errno;

	/// Creates a new instance from the given string.
	///
	/// If the total length of the path is longer than [`limits::PATH_MAX`], the function returns
	/// an error ([`errno::ENAMETOOLONG`]).
	fn try_from(s: &[u8; N]) -> EResult<Self> {
		Self::try_from(s.as_slice())
	}
}

impl TryFrom<&[u8]> for PathBuf {
	type Error = Errno;

	/// Creates a new instance from the given string.
	///
	/// If the total length of the path is longer than [`limits::PATH_MAX`], the function returns
	/// an error ([`errno::ENAMETOOLONG`]).
	fn try_from(s: &[u8]) -> EResult<Self> {
		if s.len() > limits::PATH_MAX {
			return Err(errno!(ENAMETOOLONG));
		}
		Ok(Self(String::try_from(s)?))
	}
}

impl AsRef<Path> for PathBuf {
	fn as_ref(&self) -> &Path {
		Path::new_unchecked(self.0.as_bytes())
	}
}

impl Borrow<Path> for PathBuf {
	fn borrow(&self) -> &Path {
		self.as_ref()
	}
}

impl Deref for PathBuf {
	type Target = Path;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<'p> FromIterator<Component<'p>> for CollectResult<PathBuf> {
	fn from_iter<T: IntoIterator<Item = Component<'p>>>(iter: T) -> Self {
		let iter = iter.into_iter();
		// TODO use `collect`
		let res = (|| {
			let mut path = String::new();
			for c in iter {
				path.push_str(c)?;
			}
			Ok(PathBuf::new_unchecked(path))
		})();
		Self(res)
	}
}

impl fmt::Display for PathBuf {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(self.as_ref(), f)
	}
}

impl fmt::Debug for PathBuf {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self.as_ref(), f)
	}
}

/// Borrowed file path.
#[derive(Eq, Hash, PartialEq)]
// repr(transparent) is required for the `new` function to work correctly
#[repr(transparent)]
pub struct Path([u8]);

impl Path {
	/// Creates a new path to root.
	pub fn root() -> &'static Self {
		Self::new_unchecked(&[])
	}

	/// Creates a new instance from the given string.
	///
	/// If the total length of the path is longer than [`limits::PATH_MAX`], the function returns
	/// an error ([`errno::ENAMETOOLONG`]).
	pub fn new<S: AsRef<[u8]> + ?Sized>(s: &S) -> EResult<&Self> {
		let slice = s.as_ref();
		if slice.len() <= limits::PATH_MAX {
			Ok(Self::new_unchecked(slice))
		} else {
			Err(errno!(ENAMETOOLONG))
		}
	}

	/// Creates a new instance from the given string without checking its length.
	pub fn new_unchecked<S: AsRef<[u8]> + ?Sized>(s: &S) -> &Self {
		unsafe { &*(s.as_ref() as *const [u8] as *const Self) }
	}

	/// Returns the length of the path in bytes.
	pub fn len(&self) -> usize {
		self.0.len()
	}

	/// Tells whether the path is empty.
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Tells whether the path is absolute.
	pub fn is_absolute(&self) -> bool {
		self.0.first().cloned() == Some(PATH_SEPARATOR)
	}

	/// Returns slice of the bytes representation of the path.
	pub fn as_bytes(&self) -> &[u8] {
		&self.0
	}

	/// Clones the path and returns a [`PathBuf`].
	pub fn to_path_buf(&self) -> AllocResult<PathBuf> {
		PathBuf::try_from(self)
	}

	/// Joins the path with another.
	///
	/// If `path` is absolute, it replaces the current path.
	///
	/// The function does not check path length and allows longer paths than [`limits::PATH_MAX`].
	pub fn join<P: AsRef<Path>>(&self, path: P) -> AllocResult<PathBuf> {
		let path = path.as_ref();
		if path.is_absolute() {
			path.as_ref().to_path_buf()
		} else {
			self.components()
				.chain(path.components())
				.collect::<CollectResult<PathBuf>>()
				.0
		}
	}

	/// Returns an iterator over the path's components.
	pub fn components(&self) -> Components {
		Components {
			path: self,

			front: 0,
			back: self.0.len(),
		}
	}

	/// Returns the final component of the path.
	///
	/// This function returns `None` only if it terminates with root.
	pub fn file_name(&self) -> Option<&[u8]> {
		let comp = self.components().next_back()?;
		match comp {
			Component::RootDir => None,
			Component::CurDir => Some(b"."),
			Component::ParentDir => Some(b".."),
			Component::Normal(name) => Some(name),
		}
	}

	/// Returns the path without its final component.
	///
	/// This function returns `None` only if it terminates with root.
	pub fn parent(&self) -> Option<&Path> {
		let mut comps = self.components();
		let last = comps.next_back();
		last.and_then(move |p| match p {
			Component::RootDir => None,
			_ => Some(comps.as_path()),
		})
	}

	/// Tells whether the path starts with the given `base`.
	pub fn starts_with<P: AsRef<Path>>(&self, base: P) -> bool {
		let base = base.as_ref();
		self.components()
			.zip(base.components())
			.all(|(path, base)| path == base)
	}

	/// Strips the path from the given `prefix` and returns the remaining components.
	///
	/// If the path does not start with the given `prefix`, the function returns `None`.
	pub fn strip_prefix<P: AsRef<Path>>(&self, prefix: P) -> Option<&Path> {
		let prefix = prefix.as_ref();
		let slice = self.0.strip_prefix(&prefix.0)?;
		Some(Self::new_unchecked(slice))
	}
}

impl AsRef<Path> for Path {
	fn as_ref(&self) -> &Path {
		self
	}
}

impl fmt::Display for Path {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&DisplayableStr(&self.0), f)
	}
}

impl fmt::Debug for Path {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&DisplayableStr(&self.0), f)
	}
}

/// A component of a path.
#[derive(Clone, Eq, PartialEq)]
pub enum Component<'p> {
	/// The root directory (heading `/`).
	RootDir,
	/// The current directory (`.`).
	CurDir,
	/// The parent directory (`..`).
	ParentDir,
	/// A normal component.
	Normal(&'p [u8]),
}

impl<'s> From<&'s [u8]> for Component<'s> {
	/// Returns the component corresponding to the given string representation.
	///
	/// The given string should not contain the `/` character, thus it cannot produce a `RootDir`
	/// variant.
	fn from(name: &'s [u8]) -> Self {
		match name {
			b"." => Self::CurDir,
			b".." => Self::ParentDir,
			name => Self::Normal(name),
		}
	}
}

impl<'p> AsRef<[u8]> for Component<'p> {
	fn as_ref(&self) -> &[u8] {
		match self {
			Component::RootDir => b"/",
			Component::CurDir => b".",
			Component::ParentDir => b"..",
			Component::Normal(name) => name,
		}
	}
}

impl<'p> AsRef<Path> for Component<'p> {
	fn as_ref(&self) -> &Path {
		let slice: &[u8] = self.as_ref();
		Path::new_unchecked(slice)
	}
}

impl<'p> fmt::Debug for Component<'p> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		match self {
			Component::RootDir => write!(f, "RootDir"),
			Component::CurDir => write!(f, "CurDir"),
			Component::ParentDir => write!(f, "ParentDir"),
			Component::Normal(name) => write!(f, "Normal({})", DisplayableStr(name)),
		}
	}
}

/// Iterator over a path's components.
pub struct Components<'p> {
	/// The path over which the iterator works.
	path: &'p Path,

	/// The current front offset.
	front: usize,
	/// The current back offset.
	back: usize,
}

impl<'p> Components<'p> {
	/// Tells whether the iterator is finished.
	fn is_finished(&self) -> bool {
		self.front >= self.back
	}

	/// Returns a slice to the inner representation of the remaining data in the iterator.
	pub fn as_slice(&self) -> &'p [u8] {
		&self.path.as_bytes()[self.front..self.back]
	}

	/// Returns a path representing the remaining data in the iterator.
	pub fn as_path(&self) -> &'p Path {
		Path::new_unchecked(self.as_slice())
	}

	fn next_impl(&mut self, backwards: bool) -> Option<Component<'p>> {
		// Get length of the next component
		let comp_len = loop {
			// Assert the fuse invariant
			if self.is_finished() {
				return None;
			}
			let slice = self.as_slice();
			// Get offset to the next separator
			let comp_len = if !backwards {
				slice.iter().position(|c| *c == PATH_SEPARATOR)
			} else {
				slice.iter().rev().position(|c| *c == PATH_SEPARATOR)
			}
			.unwrap_or(slice.len());
			if comp_len > 0 {
				break comp_len;
			}
			// The current character is a separator
			let root = if !backwards {
				self.front += 1;
				self.front == 1
			} else {
				self.back -= 1;
				self.back == 0
			};
			// If beginning with a separator, return root
			if root {
				return Some(Component::RootDir);
			}
		};
		let slice = self.as_slice();
		// Return component
		let comp_slice = if !backwards {
			self.front += comp_len;
			&slice[..comp_len]
		} else {
			self.back -= comp_len;
			&slice[(slice.len() - comp_len)..]
		};
		Some(Component::from(comp_slice))
	}
}

impl<'p> Iterator for Components<'p> {
	type Item = Component<'p>;

	fn next(&mut self) -> Option<Self::Item> {
		self.next_impl(false)
	}
}

impl<'p> DoubleEndedIterator for Components<'p> {
	fn next_back(&mut self) -> Option<Self::Item> {
		self.next_impl(true)
	}
}

impl<'p> FusedIterator for Components<'p> {}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn path_absolute0() {
		assert!(Path::new(b"/").unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute1() {
		assert!(Path::new(b"/.").unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute2() {
		assert!(!Path::new(b".").unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute3() {
		assert!(!Path::new(b"..").unwrap().is_absolute());
	}

	#[test_case]
	fn path_absolute4() {
		assert!(!Path::new(b"./").unwrap().is_absolute());
	}

	#[test_case]
	fn components() {
		// Absolute
		let path = Path::new(b"/etc/passwd").unwrap();
		let mut iter = path.components();
		assert_eq!(iter.next(), Some(Component::RootDir));
		assert_eq!(iter.next(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next(), Some(Component::Normal(b"passwd")));
		assert_eq!(iter.next(), None);

		// Relative
		let path = Path::new(b"etc/passwd").unwrap();
		let mut iter = path.components();
		assert_eq!(iter.next(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next(), Some(Component::Normal(b"passwd")));
		assert_eq!(iter.next(), None);

		// Relative with `.` and `..`
		let path = Path::new(b"etc/./../etc/passwd").unwrap();
		let mut iter = path.components();
		assert_eq!(iter.next(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next(), Some(Component::CurDir));
		assert_eq!(iter.next(), Some(Component::ParentDir));
		assert_eq!(iter.next(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next(), Some(Component::Normal(b"passwd")));
		assert_eq!(iter.next(), None);
	}

	#[test_case]
	fn components_back() {
		// Absolute
		let path = Path::new(b"/etc/passwd").unwrap();
		let mut iter = path.components();
		assert_eq!(iter.next_back(), Some(Component::Normal(b"passwd")));
		assert_eq!(iter.next_back(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next_back(), Some(Component::RootDir));
		assert_eq!(iter.next_back(), None);

		// Relative
		let path = Path::new(b"etc/passwd").unwrap();
		let mut iter = path.components();
		assert_eq!(iter.next_back(), Some(Component::Normal(b"passwd")));
		assert_eq!(iter.next_back(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next_back(), None);

		// Relative with `.` and `..`
		let path = Path::new(b"etc/./../etc/passwd").unwrap();
		let mut iter = path.components();
		assert_eq!(iter.next_back(), Some(Component::Normal(b"passwd")));
		assert_eq!(iter.next_back(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next_back(), Some(Component::ParentDir));
		assert_eq!(iter.next_back(), Some(Component::CurDir));
		assert_eq!(iter.next_back(), Some(Component::Normal(b"etc")));
		assert_eq!(iter.next_back(), None);
	}
}
