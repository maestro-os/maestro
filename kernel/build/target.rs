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

//! Compilation target information.

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

	/// Returns the path to the directory containing target-specific sources.
	pub fn src(&self) -> PathBuf {
		PathBuf::from(format!("arch/{}/src/", self.name))
	}

	/// Returns the name of the architecture for the compatibility vDSO, if any.
	pub fn compat_vdso(&self) -> Option<&str> {
		(self.name == "x86_64").then_some("x86")
	}
}
