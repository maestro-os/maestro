//! Some parts of the kernel are implemented in C and assembly language. Those parts are compiled
//! by the code present in this module.

use super::util;
use crate::target::Target;
use std::env;
use std::io;
use std::path::Path;
use std::path::PathBuf;

/// Compiles the vDSO.
///
/// `target` is the target to compile for.
pub fn compile_vdso(target: &Target) {
	let file = PathBuf::from(format!("vdso/{}.s", target.get_name()));

	cc::Build::new()
		.flag("-nostdlib")
		.flag("-ffreestanding")
		.flag("-mno-red-zone")
		.flag("-Wall")
		.flag("-Wextra")
		.flag("-Werror")
		.pic(true)
		.flag("-Tvdso/linker.ld")
		.shared_flag(true)
		.target(target.get_triplet())
		.file(&file)
		.compile("vdso.so");

	println!("cargo:rerun-if-changed=vdso/linker.ld");
	println!("cargo:rerun-if-changed={}", file.display());
}

/// Compiles the C and assembly code.
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
		.flag("-fno-pic")
		.flag("-mno-red-zone")
		.flag("-Wall")
		.flag("-Wextra")
		.flag("-Werror")
		.flag(&format!("-T{}", target.get_linker_script_path().display()))
		.target(target.get_triplet())
		.debug(debug)
		.opt_level(opt_level)
		.files(files)
		.compile("libmaestro.a");

	Ok(())
}
