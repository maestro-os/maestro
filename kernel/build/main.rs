//! The build script reads the configuration file, compiles required libraries and prepares for the
//! compilation of the kernel.

pub mod compile;
pub mod config;

use build_utils::{target::Target, Env};
use config::Config;
use std::process::exit;

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
