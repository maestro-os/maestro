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

//! Some parts of the kernel are implemented in C and assembly language. Those parts are compiled
//! by the code present in this module.

use crate::{target::Target, util::list_c_files, Env};
use std::{
	io,
	path::{Path, PathBuf},
	process::{exit, Command},
};

/// Compiles the vDSO.
pub fn compile_vdso(env: &Env, target: &Target) -> io::Result<()> {
	let file = PathBuf::from(format!("vdso/{}.s", target.name));
	println!("cargo:rerun-if-changed=vdso/linker.ld");
	println!("cargo:rerun-if-changed={}", file.display());
	// The path to the shared library to be compiled
	let out_path = env
		.manifest_dir
		.join(format!("target/{}/{}", target.name, env.profile))
		.join("vdso.so");
	// Compile
	let status = Command::new("clang")
		.arg("-Tvdso/linker.ld")
		.arg("-nostdlib")
		.arg("-Wall")
		.arg("-Wextra")
		.arg("-Werror")
		.arg("-fPIC")
		.arg("-target")
		.arg(&target.triplet)
		.arg("-shared")
		.arg(file)
		.arg("-o")
		.arg(&out_path)
		.status()?;
	if !status.success() {
		exit(1);
	}
	// Pass vDSO path to the rest of the codebase
	println!("cargo:rustc-env=VDSO_PATH={}", out_path.display());
	Ok(())
}

/// Compiles the C and assembly code that are parts of the kernel's codebase.
pub fn compile_c(env: &Env, target: &Target) -> io::Result<()> {
	let files = list_c_files(Path::new("src"))?;
	for f in &files {
		println!("cargo:rerun-if-changed={}", f.display());
	}
	cc::Build::new()
		.flag("-nostdlib")
		.flag("-ffreestanding")
		.flag("-fno-stack-protector")
		.flag("-mno-red-zone")
		.flag("-Wall")
		.flag("-Wextra")
		.flag("-Werror")
		.pic(false)
		.target(&target.triplet)
		.flag(&format!("-T{}", target.get_linker_script_path().display()))
		.debug(env.is_debug())
		.opt_level(env.opt_level)
		.files(files)
		.compile("casm");
	println!("cargo:rustc-link-arg=-lcasm");
	Ok(())
}
