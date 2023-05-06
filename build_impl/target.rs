//! TODO doc

use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

/// Structure representing the content of the target JSON file.
///
/// This structure contains only the fields that are of interest.
#[derive(Deserialize)]
pub struct TargetFile {
	/// The LLVM target triplet.
	#[serde(rename = "llvm-target")]
	llvm_target: String,
	/// The linker to use.
	linker: String,
}

/// Structure representing a build target.
pub struct Target {
	/// The name of the target.
	name: String,
	/// The target triplet.
	triplet: String,

	/// The linker to use.
	linker: String,
}

impl Target {
	/// Returns the selected triplet according to environment variables.
	///
	/// If no target has been provided, the function returns `None`.
	pub fn from_env() -> io::Result<Option<Self>> {
		// Get target file path
		let Ok(arch) = env::var("CARGO_CFG_TARGET_ARCH") else {
            return Ok(None);
        };
		let target_path = PathBuf::from(format!("arch/{arch}/target.json"));

		// Read and parse target file
		let content = fs::read_to_string(target_path)?;
		let content: TargetFile =
			serde_json::from_str(&content).map_err(|e| io::Error::from(e))?;

		Ok(Some(Target {
			name: arch,
			triplet: content.llvm_target,

			linker: content.linker,
		}))
	}

	/// Returns the name of the target.
	pub fn get_name(&self) -> &str {
		&self.name
	}

	/// Returns the path to the linker script of the target.
	pub fn get_linker_script_path(&self) -> PathBuf {
		PathBuf::from(format!("arch/{}/linker.ld", self.name))
	}

	/// Returns the target's triplet.
	pub fn get_triplet(&self) -> &str {
		&self.triplet
	}

	/// Returns the linker to use.
	pub fn get_linker(&self) -> &str {
		&self.linker
	}
}
