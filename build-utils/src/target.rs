//! TODO doc

use crate::Env;
use serde::Deserialize;
use std::{fs, io, path::PathBuf};

/// The content of the target JSON file.
///
/// This structure contains only the fields that are of interest.
#[derive(Deserialize)]
struct TargetFile {
	/// The LLVM target triplet.
	#[serde(rename = "llvm-target")]
	llvm_target: String,
}

/// A build target.
pub struct Target<'s> {
	/// The name of the target.
	pub name: &'s str,
	/// The target triplet.
	pub triplet: String,
}

impl<'s> Target<'s> {
	/// Returns the selected triplet according to the environment.
	pub fn from_env(env: &'s Env) -> io::Result<Self> {
		// Read and parse target file
		let content = fs::read_to_string(&env.target_path)?;
		let content: TargetFile = serde_json::from_str(&content).map_err(io::Error::from)?;
		Ok(Self {
			name: &env.arch,
			triplet: content.llvm_target,
		})
	}

	/// Returns the path to the linker script of the target.
	pub fn get_linker_script_path(&self) -> PathBuf {
		PathBuf::from(format!("arch/{}/linker.ld", self.name))
	}
}
