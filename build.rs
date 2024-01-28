//! The build script reads the configuration file, compiles required libraries and prepares for the
//! compilation of the kernel.

mod build_impl;

use build_impl::*;
use config::Config;
use std::{env, io::ErrorKind, process::exit};
use target::Target;

fn main() {
	let profile = env::var("PROFILE").unwrap();
	let debug = profile == "debug";
	let opt_level: u32 = env::var("OPT_LEVEL").unwrap().parse().unwrap();

	let config = Config::read().unwrap_or_else(|e| {
		if e.kind() == ErrorKind::NotFound {
			eprintln!("Configuration file not found");
			eprintln!();
			eprintln!(
				"Please make sure the configuration file at `{}` exists`",
				config::PATH
			);
			eprintln!("An example configuration file can be found in `default.config.toml`");
		} else {
			eprintln!("Cannot read configuration file: {e}");
		}
		exit(1);
	});
	config.set_cfg(debug);

	let target = Target::from_env().unwrap_or_else(|e| {
		eprintln!("Cannot retrieve target: {e}");
		exit(1);
	});

	compile::compile_c(&target, debug, opt_level).unwrap_or_else(|e| {
		eprintln!("Compilation failed: {e}");
		exit(1);
	});
	compile::compile_vdso(&target, &profile).unwrap_or_else(|e| {
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
