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
	pub fn from_env() -> io::Result<Self> {
		// Get target file path
		// Unwrapping is safe because a default target is specified in `.cargo/config.toml`
		let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

		// Read and parse target file
		let target_path = PathBuf::from(format!("arch/{arch}/{arch}.json"));
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
