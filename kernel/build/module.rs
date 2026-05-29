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

//! Compiles and embeds built-in kernel modules into the binary.

use crate::{Env, config::Config};
use std::{
	fs, io,
	path::{Path, PathBuf},
	process::{Command, exit},
};

/// Builds a single built-in module and returns the path to the compiled `.so`.
fn build_module(
	env: &Env,
	mod_dir: &Path,
	module_name: &str,
	libkernel: &Path,
) -> io::Result<PathBuf> {
	let so_path = mod_dir.join(format!(
		"target/{}/{}/lib{module_name}.so",
		env.arch, env.profile
	));

	let deps_dir = libkernel
		.parent()
		.unwrap_or_else(|| {
			eprintln!("error: libkernel path has no parent directory");
			exit(1);
		})
		.join("deps");
	let host_deps_dir = env
		.manifest_dir
		.join(format!("target/{}/deps", env.profile));
	let target_path = env.target_path.to_str().unwrap_or_else(|| {
		eprintln!("error: target path is not valid UTF-8");
		exit(1);
	});

	let status = Command::new("cargo")
		.arg("build")
		.args(["--target", target_path])
		.args(env.is_release().then_some("--release"))
		// \x1f (ASCII unit separator) is the delimiter required by CARGO_ENCODED_RUSTFLAGS.
		.env(
			"CARGO_ENCODED_RUSTFLAGS",
			format!(
				"--extern\x1fkernel={}\x1f-L\x1f{}\x1f-L\x1f{}",
				libkernel.display(),
				deps_dir.display(),
				host_deps_dir.display()
			),
		)
		.current_dir(mod_dir)
		.status()?;

	if !status.success() {
		eprintln!("error: failed to compile built-in module '{module_name}'");
		exit(1);
	}

	// Emit a `cargo:rerun-if-changed` directive for the compiled `.so` so that
	// the build script re-runs if the module is rebuilt outside of this script.
	println!("cargo:rerun-if-changed={}", so_path.display());

	Ok(so_path)
}

/// Generates `builtin_modules.rs`, embedding each `.so` as an aligned byte array.
fn generate(so_paths: &[PathBuf], out_dir: &Path) -> io::Result<()> {
	let mut statics = String::new();
	let mut refs = Vec::new();
	for (i, path) in so_paths.iter().enumerate() {
		let path = path.display();
		statics.push_str(&format!(
			"static BUILTIN_MODULE_{i}: AlignedModule<{{ include_bytes!(\"{path}\").len() }}> \
			 = AlignedModule(*include_bytes!(\"{path}\"));\n"
		));
		refs.push(format!("&BUILTIN_MODULE_{i}.0"));
	}
	let refs_list = refs.join(",\n    ");

	// `include_bytes!` embeds data at 1-byte alignment, but ELF header parsing
	// requires at least 8-byte alignment (for u64 fields in ELF64ELFHeader).
	// Wrapping each embedded file in `#[repr(align(8))]` ensures the bytes
	// are properly aligned when passed to `ELFParser::from_slice`.
	let generated = if so_paths.is_empty() {
		"static BUILTIN_MODULES: &[&[u8]] = &[];\n".to_string()
	} else {
		format!(
			"#[repr(align(8))]\nstruct AlignedModule<const N: usize>([u8; N]);\n\n\
			 {statics}\nstatic BUILTIN_MODULES: &[&[u8]] = &[\n    {refs_list}\n];\n"
		)
	};
	fs::write(out_dir.join("builtin_modules.rs"), generated)?;

	Ok(())
}

/// Compiles and embeds built-in modules into the kernel binary.
///
/// Modules listed in `config.modules.builtin` are compiled as `.so` shared libraries
/// and embedded as byte arrays. At runtime, `load_builtin_modules()` loads them using
/// the existing `Module::load()` infrastructure.
///
/// # First-build caveat
///
/// Module compilation requires `libkernel.rlib` (modules link against the kernel).
/// This file is produced by the kernel build itself, so on the very first `cargo build`
/// it doesn't exist yet. In that case, embedding is skipped with a warning, and the
/// kernel is built without built-in modules.
///
/// Use `cargo xtask build-kernel` from the repo root to handle this automatically (it runs two
/// passes under the hood). Alternatively, run `cargo build` a second time manually.
pub fn embed_builtin_modules(env: &Env, config: &Config) -> io::Result<()> {
	let workspace_root = env.manifest_dir.parent().unwrap_or_else(|| {
		eprintln!("error: manifest directory has no parent");
		exit(1);
	});
	let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_else(|e| {
		eprintln!("error: OUT_DIR not set: {e}");
		exit(1);
	}));

	let libkernel = env.manifest_dir.join(format!(
		"target/{}/{}/libkernel.rlib",
		env.arch, env.profile
	));
	// Tell Cargo to re-run this build script when libkernel.rlib appears or changes.
	// Without this, the second `cargo build` would skip the build script (and leave
	// builtin_modules.rs empty) because Cargo has no way to know that the rlib was created.
	println!("cargo:rerun-if-changed={}", libkernel.display());

	let mut so_paths: Vec<PathBuf> = Vec::new();
	for module_name in config.builtin_modules() {
		let mod_dir = workspace_root.join(format!("mod/{module_name}"));

		// Trigger rerun if module source or manifest changes, even when libkernel is
		// absent, so the next `cargo build` picks up source edits.
		println!("cargo:rerun-if-changed={}", mod_dir.join("src").display());
		println!(
			"cargo:rerun-if-changed={}",
			mod_dir.join("Cargo.toml").display()
		);

		if !libkernel.exists() {
			eprintln!(
				"cargo:warning=libkernel.rlib not found — cannot compile built-in module \
				 '{module_name}'.\nUse `cargo xtask build-kernel` from the repo root, or run `cargo build` once more."
			);
			continue;
		}

		so_paths.push(build_module(env, &mod_dir, module_name, &libkernel)?);
	}

	generate(&so_paths, &out_dir)
}
