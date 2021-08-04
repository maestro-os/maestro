//! A kernel module is an executable that is loaded in kernelspace in order to handle a specific
//! feature. The some advantages of that system is a lighter kernel with clearer code and it
//! allows to load only the parts that are required by the current system.
//!
//! There's a distinction between a Module and a Kernel module:
//! - Module: A Rust module, part of the structure of the code.
//! - Kernel module: A piece of software to be loaded at runtime in kernelspace.

pub mod version;

use crate::elf::ELFParser;
use crate::errno::Errno;
use crate::util::container::string::String;
use version::Version;

/// Structure representing a kernel module.
pub struct Module {
	/// The module's name.
	name: String,
	/// The module's version.
	version: Version,

	// TODO Store a pointer to the module image

	/// Pointer to the module's destructor.
	fini: Option<fn()>,
}

impl Module {
	/// Loads a kernel module from the given image.
	pub fn load(image: &[u8]) -> Result<Self, Errno> {
		let _parser = ELFParser::new(image)?;

		// TODO
		Ok(Self {
			name: String::from("TODO")?,
			version: Version {
				major: 0,
				minor: 0,
				patch: 0,
			},

			fini: None,
		})
	}

	/// Returns the name of the module.
	pub fn get_name(&self) -> &String {
		&self.name
	}

	/// Returns the version of the module.
	pub fn get_version(&self) -> &Version {
		&self.version
	}
}

impl Drop for Module {
	fn drop(&mut self) {
		if let Some(fini) = self.fini {
			fini();
		}
	}
}
