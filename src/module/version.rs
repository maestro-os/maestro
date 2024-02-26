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

//! The module implements a Version structure.
//!
//! A version is divided into the following component:
//! - Major: Version including breaking changes
//! - Minor: Version including new features
//! - Patch: Version including bug fixes and optimizations

use core::{
	cmp::Ordering,
	fmt::{Display, Error, Formatter},
};

/// Structure representing a version.
#[derive(Clone, Copy, Debug, Eq)]
pub struct Version {
	/// The major version
	pub major: u16,
	/// The minor version
	pub minor: u16,
	/// The patch version
	pub patch: u16,
}

impl Version {
	/// Creates a new instance.
	pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
		Self {
			major,
			minor,
			patch,
		}
	}

	// FIXME: this function currently cannot be written cleanly since const functions are not very
	// advanced in Rust. When improvements will be made, rewrite it
	/// Parses a version from the given string.
	///
	/// If the version is invalid, the function returns `None`.
	pub const fn parse(s: &str) -> Option<Self> {
		let mut nbrs: [u16; 3] = [0; 3];
		let mut n = 0;

		let bytes = s.as_bytes();
		let mut i = 0;
		while i < bytes.len() {
			if !(bytes[i] as char).is_ascii_digit() {
				return None;
			}

			// Parse number
			let mut nbr: u16 = 0;
			while i < bytes.len() && (bytes[i] as char).is_ascii_digit() {
				nbr *= 10;
				nbr += (bytes[i] - b'0') as u16;
				i += 1;
			}

			nbrs[n] = nbr;
			n += 1;

			if i < bytes.len() && bytes[i] == b'.' && n >= nbrs.len() {
				return None;
			}
			i += 1;
		}

		if n >= nbrs.len() {
			Some(Self {
				major: nbrs[0],
				minor: nbrs[1],
				patch: nbrs[2],
			})
		} else {
			None
		}
	}
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> Ordering {
		self.major
			.cmp(&other.major)
			.then_with(|| self.minor.cmp(&other.minor))
			.then_with(|| self.patch.cmp(&other.patch))
	}
}

impl PartialOrd for Version {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for Version {
	fn eq(&self, other: &Self) -> bool {
		self.major == other.major && self.minor == other.minor && self.patch == other.patch
	}
}

impl Display for Version {
	fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), Error> {
		write!(fmt, "{}.{}.{}", self.major, self.minor, self.patch)
	}
}

/// Structure representing a dependency of a module.
#[derive(Clone)]
pub struct Dependency {
	/// The name of the module
	pub name: &'static str,
	/// The version.
	pub version: Version,
	/// The constraint on the version.
	pub constraint: Ordering,
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn version_parse() {
		assert_eq!(Version::parse(""), None);
		assert_eq!(Version::parse("."), None);
		assert_eq!(Version::parse("0."), None);
		assert_eq!(Version::parse("0.0"), None);
		assert_eq!(Version::parse("0.0."), None);
		assert_eq!(Version::parse("0..0"), None);
		assert_eq!(Version::parse(".0.0"), None);
		assert_eq!(Version::parse("0.0.0."), None);
		assert_eq!(Version::parse("0.0.0.0"), None);

		assert_eq!(
			Version::parse("0.0.0"),
			Some(Version {
				major: 0,
				minor: 0,
				patch: 0,
			})
		);
		assert_eq!(
			Version::parse("1.0.0"),
			Some(Version {
				major: 1,
				minor: 0,
				patch: 0,
			})
		);
		assert_eq!(
			Version::parse("0.1.0"),
			Some(Version {
				major: 0,
				minor: 1,
				patch: 0,
			})
		);
		assert_eq!(
			Version::parse("0.0.1"),
			Some(Version {
				major: 0,
				minor: 0,
				patch: 1,
			})
		);

		assert_eq!(
			Version::parse("1.2.3"),
			Some(Version {
				major: 1,
				minor: 2,
				patch: 3,
			})
		);
	}
}
