use build_utils::{list_c_files, target::Target, Env};
use std::{io, path::Path};

fn main() -> io::Result<()> {
	// Env
	let env = Env::get();
	let target = Target::from_env(&env)?;
	// List files
	let files = list_c_files(Path::new("src"))?;
	for f in &files {
		println!("cargo:rerun-if-changed={}", f.display());
	}
	// Build
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
		.compile("libcasm.a");
	Ok(())
}
