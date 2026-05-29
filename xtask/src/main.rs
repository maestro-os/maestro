/*
 * Copyright 2026 Luc Lenôtre
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

//! Build orchestrator for the Maestro kernel.
//!
//! Run with `cargo xtask build-kernel [--release]`.
//! Handles the two-pass build required when built-in modules are configured.

use std::{
	path::{Path, PathBuf},
	process::{Command, exit},
};

fn kernel_dir() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR"))
		.parent()
		.expect("xtask has no parent directory")
		.join("kernel")
}

fn cargo_build(kernel_dir: &Path, extra_args: &[String]) {
	let status = Command::new("cargo")
		.arg("build")
		.args(extra_args)
		.current_dir(kernel_dir)
		.status()
		.expect("failed to spawn cargo");
	if !status.success() {
		exit(status.code().unwrap_or(1));
	}
}

fn cmd_build_kernel(args: &[String]) {
	let dir = kernel_dir();
	// First pass: compiles the kernel, producing libkernel.rlib.
	// If built-in modules are configured, the build script skips them with a warning
	// because libkernel.rlib does not yet exist at build-script time.
	cargo_build(&dir, args);
	// Second pass: build script re-runs (triggered by rerun-if-changed=libkernel.rlib),
	// compiles modules against the now-available rlib, and embeds them.
	// If no built-in modules are configured this pass is a fast no-op.
	cargo_build(&dir, args);
}

fn print_help() {
	eprintln!(
		"Usage: cargo xtask <command> [options]\n\
		 \n\
		 Commands:\n\
		   build-kernel [--release]   Build the kernel, embedding any configured built-in modules\n\
		   help                       Show this message"
	);
}

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	match args.first().map(String::as_str) {
		Some("build-kernel") => cmd_build_kernel(&args[1..]),
		Some("help") | Some("--help") | Some("-h") | None => print_help(),
		Some(cmd) => {
			eprintln!("error: unknown command '{cmd}'");
			print_help();
			exit(1);
		}
	}
}
