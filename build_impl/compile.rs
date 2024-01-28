//! Some parts of the kernel are implemented in C and assembly language. Those parts are compiled
//! by the code present in this module.

use super::util;
use crate::target::Target;
use std::{
	io,
	path::{Path, PathBuf},
	process::{exit, Command},
};

/// Compiles the vDSO.
pub fn compile_vdso(target: &Target, profile: &str) -> io::Result<()> {
	let file = PathBuf::from(format!("vdso/{}.s", target.name));

	println!("cargo:rerun-if-changed=vdso/linker.ld");
	println!("cargo:rerun-if-changed={}", file.display());

	let out_dir = PathBuf::from(format!("target/{}/{}/", target.name, profile));
	let out_dir = out_dir.canonicalize()?;

	// The path to the shared library to be compiled
	let out_path = out_dir.join("vdso.so");

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
pub fn compile_c(target: &Target, debug: bool, opt_level: u32) -> io::Result<()> {
	let files = util::list_c_files(Path::new("src"))?;
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
		//.flag("-Werror")
		.pic(false)
		.target(&target.triplet)
		.flag(&format!("-T{}", target.get_linker_script_path().display()))
		.debug(debug)
		.opt_level(opt_level)
		.files(files)
		.compile("libcasm.a");

	Ok(())
}
