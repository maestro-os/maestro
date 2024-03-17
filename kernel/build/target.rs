//! TODO doc

use serde::Deserialize;
use std::{env, fs, io, path::PathBuf};

/// The content of the target JSON file.
///
/// This structure contains only the fields that are of interest.
#[derive(Deserialize)]
pub struct TargetFile {
	/// The LLVM target triplet.
	#[serde(rename = "llvm-target")]
	llvm_target: String,
}

/// A build target.
pub struct Target {
	/// The name of the target.
	pub name: String,
	/// The target triplet.
	pub triplet: String,
}

impl Target {
	/// Returns the selected triplet according to environment variables.
	///
	/// If no target has been provided, the function returns `None`.
	pub fn from_env(manifest_dir: &str) -> io::Result<Self> {
		// Get target file path
		// Unwrapping is safe because a default target is specified in `.cargo/config.toml`
		let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
		// Read and parse target file
		let target_path = PathBuf::from(format!("{manifest_dir}/../arch/{arch}/{arch}.json"));
		let content = fs::read_to_string(target_path)?;
		let content: TargetFile = serde_json::from_str(&content).map_err(io::Error::from)?;
		Ok(Self {
			name: arch,
			triplet: content.llvm_target,
		})
	}

	/// Returns the path to the linker script of the target.
	pub fn get_linker_script_path(&self) -> PathBuf {
		PathBuf::from(format!("arch/{}/linker.ld", self.name))
	}
}
