//! Some parts of the kernel are implemented in C and assembly language. Those parts are compiled
//! by the code present in this module.

use super::util;
use crate::target::Target;
use std::env;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;

/// Compiles the vDSO.
///
/// `target` is the target to compile for.
pub fn compile_vdso(target: &Target) {
	let file = PathBuf::from(format!("vdso/{}.s", target.get_name()));

	println!("cargo:rerun-if-changed=vdso/linker.ld");
	println!("cargo:rerun-if-changed={}", file.display());

	let profile = env::var("PROFILE").unwrap();
	let out_dir = PathBuf::from(format!("target/{}/{}/", target.get_name(), profile));
	let out_dir = out_dir.canonicalize().unwrap();

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
		.arg(target.get_triplet())
		.arg("-shared")
		.arg(file)
		.arg("-o")
		.arg(&out_path)
		.status()
		.unwrap();
	if !status.success() {
		exit(1);
	}

	// Pass vDSO path to the rest of the codebase
	println!("cargo:rustc-env=VDSO_PATH={}", out_path.display());
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
		.compile("libcasm.a");

	Ok(())
}
