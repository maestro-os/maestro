//! Some parts of the kernel are implemented in C and assembly language. Those parts are compiled
//! by the code present in this module.

use crate::target::Target;
use std::env;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use super::util;

/// Compiles the vDSO.
///
/// `target` is the target to compile for.
pub fn compile_vdso(target: &Target) {
	let file = PathBuf::from(format!("vdso/{}.s", target.get_name()));

	println!("cargo:rerun-if-changed=vdso/linker.ld");
	println!("cargo:rerun-if-changed={}", file.display());

	// Compile as static library
	cc::Build::new()
		.no_default_flags(true)
		.flag("-nostdlib")
		.flag("-ffreestanding")
		.flag("-mno-red-zone")
		.flag("-Wall")
		.flag("-Wextra")
		//.flag("-Werror")
		.pic(true)
		.target(target.get_triplet())
		.file(&file)
		.compile("vdso");

	let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

	// The path to the static library
	let static_path = out_dir.join("libvdso.a");
	// The path to the shared library to be compiled
	let shared_path = out_dir.join("vdso.so");

	// Link into a shared library
	//
	// A second pass for linking is required since the crate `cc` can only build static libraries
	let status = Command::new(target.get_linker())
		.arg("-Tvdso/linker.ld")
		.arg("-shared")
		.arg(static_path)
		.arg("-o")
		.arg(&shared_path)
		.status()
		.unwrap();
	if !status.success() {
		// TODO
		todo!();
	}

	// Pass vDSO path to the rest of the codebase
	println!("cargo:rustc-env=VDSO_PATH={}", shared_path.display());
}

/// Compiles the C and assembly code that are parts of the kernel's codebase.
///
/// `target` is the target to compile for.
pub fn compile_c(target: &Target) -> io::Result<()> {
	let debug = env::var("PROFILE").unwrap() == "debug";
	let opt_level: u32 = env::var("OPT_LEVEL").unwrap().parse().unwrap();

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
		.target(target.get_triplet())
		.flag(&format!("-T{}", target.get_linker_script_path().display()))
		.debug(debug)
		.opt_level(opt_level)
		.files(files)
		.compile("libmaestro.a");

	Ok(())
}
