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

use crate::{Env, target::Target, util::list_c_files};
use std::{
	io,
	path::{Path, PathBuf},
	process::{Command, exit},
};

fn compile_vdso_impl(
	env: &Env,
	target: &Target,
	compat_name: Option<&str>,
) -> io::Result<PathBuf> {
	let arch_name = compat_name.unwrap_or(target.name);
	let src = PathBuf::from(format!("vdso/{arch_name}.s"));
	println!("cargo:rerun-if-changed={}", src.display());
	let out_path = env.manifest_dir.join(format!(
		"target/{}/{}/vdso-{arch_name}.so",
		target.name, env.profile
	));
	let mut cmd = Command::new("clang");
	cmd.arg("-Tvdso/linker.ld")
		.arg("-nostdlib")
		.arg("-Wall")
		.arg("-Wextra")
		.arg("-Werror")
		.arg("-fPIC")
		.arg("-target")
		.arg(&target.triplet)
		.arg("-shared")
		.arg(src)
		.arg("-o")
		.arg(&out_path);
	if compat_name.is_some() {
		cmd.arg("-m32");
	}
	let status = cmd.status()?;
	if !status.success() {
		exit(1);
	}
	Ok(out_path)
}

/// Compiles the vDSO.
pub fn compile_vdso(env: &Env, target: &Target) -> io::Result<()> {
	println!("cargo:rerun-if-changed=vdso/linker.ld");
	// Compile main vDSO and pass it to the codebase
	let out_path = compile_vdso_impl(env, target, None)?;
	println!("cargo:rustc-env=VDSO_PATH={}", out_path.display());
	if let Some(name) = target.compat_vdso() {
		// Compile compat vDSO and pass it to the codebase
		let out_path = compile_vdso_impl(env, target, Some(name))?;
		println!("cargo:rustc-env=VDSO_COMPAT_PATH={}", out_path.display());
	}
	Ok(())
}

/// Compiles the C and assembly code that are parts of the kernel's codebase.
pub fn compile_c(env: &Env, target: &Target) -> io::Result<()> {
	let files: Vec<PathBuf> = list_c_files(Path::new("src"))?
		.into_iter()
		.chain(list_c_files(&target.src())?)
		.collect();
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
		.flag("-Wno-unused-command-line-argument")
		.flag("-Werror")
		.pic(false)
		.target(&target.triplet)
		.debug(env.is_debug())
		.opt_level(env.opt_level)
		.files(files)
		.compile("casm");
	// Necessary to get access from dependencies
	println!("cargo:rustc-link-arg=-lcasm");
	Ok(())
}
