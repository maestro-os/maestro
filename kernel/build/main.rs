//! The build script reads the configuration file, compiles required libraries and prepares for the
//! compilation of the kernel.

pub mod compile;
pub mod config;
pub mod target;
pub mod util;

use crate::{config::Config, target::Target};
use std::{env, path::PathBuf, process::exit};

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
	let target = Target::from_env(&env).unwrap_or_else(|e| {
		eprintln!("Cannot retrieve target: {e}");
		exit(1);
	});
	let config = Config::read().unwrap_or_else(|e| {
		eprintln!("Failed to read build configuration file: {e}");
		exit(1);
	});
	config.set_cfg(env.is_debug());
	// Compile
	compile::compile_c(&env, &target).unwrap_or_else(|e| {
		eprintln!("Compilation failed: {e}");
		exit(1);
	});
	compile::compile_vdso(&env, &target).unwrap_or_else(|e| {
		eprintln!("vDSO compilation failed: {e}");
		exit(1);
	});
	// Add the linker script
	println!(
		"cargo:rerun-if-changed={}",
		target.get_linker_script_path().display()
	);
	println!(
		"cargo:rustc-link-arg=-T{}",
		target.get_linker_script_path().display()
	);
}
