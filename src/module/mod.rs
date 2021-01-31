/// TODO doc

use core::cmp::Ordering;

/// Structure representing the version of a module.
/// The version is divided into the following component:
/// - Major: Version including breaking changes
/// - Minor: Version including new features
/// - Patch: Version including bug fixes and optimizations
#[derive(Eq)]
pub struct Version {
	/// The major version
	pub major: u16,
	/// The minor version
	pub minor: u16,
	/// The patch version
	pub patch: u16,
}

impl Version {
	/// Compares current version with the given one.
	fn cmp(&self, other: &Self) -> Ordering {
		let mut ord = self.major.cmp(&other.major);
		if ord != Ordering::Equal {
			return ord;
		}

		ord = self.minor.cmp(&other.minor);
		if ord != Ordering::Equal {
			return ord;
		}

		self.patch.cmp(&other.patch)
	}

	// TODO to_string
}

impl Ord for Version {
	fn cmp(&self, other: &Self) -> Ordering {
		self.cmp(other)
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

/// Structure describing a kernel module.
pub trait Module {
	/// Returns the name of the module.
	fn get_name(&self) -> &str;

	/// Returns the version of the module.
	fn get_version(&self) -> Version;

	/// Function called after the module have been loaded for initialization.
	fn init(&mut self) -> Result::<(), ()>;

	/// Function called before unloading the module.
	fn destroy(&mut self);
}
