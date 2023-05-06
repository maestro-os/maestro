//! TODO doc

mod build_impl;

use build_impl::*;
use config::Config;
use std::io::ErrorKind;
use std::process::exit;
use target::Target;

fn main() {
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
			eprintln!("Cannot read configuration file: {}", e);
		}

		exit(1);
	});
	config.set_cfg();

	let target = Target::from_env()
		.unwrap_or_else(|e| {
			eprintln!("Cannot retrieve target: {}", e);
			exit(1);
		})
		.unwrap_or_else(|| {
			eprintln!("No target specified. Please specify one with the `--target` option");
			exit(1);
		});

	compile::compile_c(&target).unwrap_or_else(|e| {
		eprintln!("Compilation failed: {}", e);
		exit(1);
	});
	compile::compile_vdso(&target);

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
