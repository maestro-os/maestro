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

//! Generates `builtin_modules.rs`, which compiles configured modules directly
//! into the kernel binary via `#[path]` declarations.

use crate::{Env, config::Config};
use serde::Deserialize;
use std::{
	fs,
	io::{self},
	path::{Path, PathBuf},
};

#[derive(Deserialize)]
struct ModCargo {
	package: ModPackage,
}

#[derive(Deserialize)]
struct ModPackage {
	version: String,
}

fn walk_rs_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
	for entry in fs::read_dir(dir)? {
		let path = entry?.path();
		if path.is_dir() {
			walk_rs_files(&path, out)?;
		} else if path.extension().is_some_and(|e| e == "rs") {
			out.push(path);
		}
	}
	Ok(())
}

fn parse_version(s: &str) -> (u16, u16, u16) {
	let mut parts = s.splitn(3, '.');
	let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
	let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
	let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
	(major, minor, patch)
}

/// Uses `#[path]` to compile each configured module directly into the kernel crate.
/// `extern crate self as kernel` at the crate root makes `kernel::` resolve without
/// a separate libkernel.rlib, so no two-pass build is needed.
pub fn embed_builtin_modules(env: &Env, config: &Config) -> io::Result<()> {
	let workspace_root = env
		.manifest_dir
		.parent()
		.expect("manifest directory has no parent");
	let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));

	let modules = config.builtin_modules();

	println!("cargo:rustc-check-cfg=cfg(maestro_builtin)");

	if modules.is_empty() {
		fs::write(
			out_dir.join("builtin_modules.rs"),
			"/// Loads built-in modules. No-op when none are configured.\npub fn load_builtin_modules() {}\n",
		)?;
		return Ok(());
	}

	// At least one module: set the cfg flag so module sources switch their
	// `extern crate` and `#[no_mangle]` attributes appropriately.
	println!("cargo:rustc-cfg=maestro_builtin");

	let mut path_decls = String::new();
	let mut load_calls = String::new();

	for name in modules {
		if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
			panic!(
				"built-in module name {name:?} is not a valid Rust identifier (only [a-zA-Z0-9_] allowed)"
			);
		}

		let mod_dir = workspace_root.join("mod").join(name);
		let src_path = mod_dir.join("src/mod.rs");
		let cargo_toml_path = mod_dir.join("Cargo.toml");

		println!("cargo:rerun-if-changed={}", cargo_toml_path.display());
		let mut rs_files = Vec::new();
		walk_rs_files(&mod_dir.join("src"), &mut rs_files)?;
		for path in rs_files {
			println!("cargo:rerun-if-changed={}", path.display());
		}

		let cargo_str = fs::read_to_string(&cargo_toml_path).unwrap_or_else(|e| {
			panic!("cannot read {}: {e}", cargo_toml_path.display());
		});
		let cargo: ModCargo = toml::from_str(&cargo_str).unwrap_or_else(|e| {
			panic!("cannot parse {}: {e}", cargo_toml_path.display());
		});
		let (major, minor, patch) = parse_version(&cargo.package.version);

		let ident = format!("_builtin_{name}");
		let src_escaped = src_path
			.to_str()
			.unwrap_or_else(|| panic!("non-UTF-8 path: {}", src_path.display()))
			.replace('\\', "\\\\")
			.replace('"', "\\\"");

		path_decls.push_str(&format!("#[path = \"{src_escaped}\"]\nmod {ident};\n\n"));
		load_calls.push_str(&format!(
			"\tif let Err(e) = Module::new_builtin(\"{name}\", Version::new({major}, {minor}, {patch}), \
			 {ident}::init, {ident}::fini).and_then(add) {{\n\
			 \t\tpanic!(\"built-in module '{name}' failed: {{e}}\");\n\
			 \t}}\n"
		));
	}

	let generated = format!(
		"{path_decls}/// Loads all modules that were compiled directly into the kernel binary.\n\
		 pub fn load_builtin_modules() {{\n\
		 \tuse crate::module::version::Version;\n\
		 {load_calls}}}\n"
	);
	fs::write(out_dir.join("builtin_modules.rs"), generated)?;

	Ok(())
}
