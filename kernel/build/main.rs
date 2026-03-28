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

//! The build script reads the configuration file, compiles required libraries and prepares for the
//! compilation of the kernel.

pub mod compile;
pub mod config;
pub mod font;
pub mod target;
pub mod util;

use crate::{config::Config, target::Target};
use std::{env, path::PathBuf};

/// The environment passed to the build script.
pub struct Env {
	/// The path to the root of the workspace.
	pub manifest_dir: PathBuf,
	/// The name of the profile used to compile the crate.
	pub profile: String,
	/// The optimization level, between `0` and `3` included.
	pub opt_level: u32,
	/// The name of the target architecture.
	pub arch: String,
	/// The path to the target file.
	pub target_path: PathBuf,
}

impl Env {
	/// Reads the current environment.
	pub fn get() -> Self {
		let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
		let profile = env::var("PROFILE").unwrap();
		let opt_level = env::var("OPT_LEVEL").unwrap().parse().unwrap();
		// Unwrapping is safe because a default target is specified in `.cargo/config.toml`
		let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
		let target_path = manifest_dir.join(format!("arch/{arch}/{arch}.json"));
		Self {
			manifest_dir,
			profile,
			opt_level,
			arch,
			target_path,
		}
	}

	/// Tells whether compiling in debug mode.
	pub fn is_debug(&self) -> bool {
		self.profile == "debug"
	}
}

fn main() {
	// Read config
	let env = Env::get();
	let target = Target::from_env(&env).expect("cannot retrieve target");
	let config = Config::read().expect("failed to read build configuration file");
	config.set_cfg(env.is_debug());
	// Build TTY font, if enabled
	if config.tty.enabled {
		font::build(&config.tty.font).expect("failed to build font");
	}
	// Compile
	compile::compile_c(&env, &target).expect("compilation failed");
	compile::compile_vdso(&env, &target).expect("vDSO compilation failed");
	// Add the linker script
	println!(
		"cargo:rerun-if-changed={}",
		target.get_linker_script_path().display()
	);
	println!(
		"cargo:rustc-link-arg=-T{}",
		target.get_linker_script_path().display()
	);
	// Prevent the linker from using very large alignments
	println!("cargo:rustc-link-arg=-zmax-page-size=0x1000");
}
