//! The module implements a Version structure.
//!
//! A version is divided into the following component:
//! - Major: Version including breaking changes
//! - Minor: Version including new features
//! - Patch: Version including bug fixes and optimizations

use core::cmp::Ordering;
use core::fmt::Display;
use core::fmt::Error;
use core::fmt::Formatter;

/// Structure representing a version.
#[derive(Clone, Debug, Eq)]
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

	/// TODO doc
	pub const fn parse(_s: &str) -> Result<Self, ()> {
		// TODO
		todo!()
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
