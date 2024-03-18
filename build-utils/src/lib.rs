//! Utility features for build scripts

pub mod target;

use std::{
	env,
	ffi::OsStr,
	fs, io,
	path::{Path, PathBuf},
};

/// The environment passed to the build script.
pub struct Env {
	/// The path to the root of the workspace.
	pub workspace_root: PathBuf,
	/// The name of the profile used to compile the crate.
	pub profile: String,
	/// The optimization level, between `0` and `3` included.
	pub opt_level: u32,
	/// The name of the target architecture.
	pub arch: String,
	/// The path to the target file.
	pub target_path: PathBuf,
}

impl Env {
	/// Reads the current environment.
	pub fn get() -> Self {
		let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
		let workspace_root = PathBuf::from(manifest_dir).parent().unwrap().to_path_buf();
		let profile = env::var("PROFILE").unwrap();
		let opt_level = env::var("OPT_LEVEL").unwrap().parse().unwrap();
		// Unwrapping is safe because a default target is specified in `.cargo/config.toml`
		let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();
		let target_path = workspace_root.join(format!("arch/{arch}/{arch}.json"));
		Self {
			workspace_root,
			profile,
			opt_level,
			arch,
			target_path,
		}
	}

	/// Tells whether compiling in debug mode.
	pub fn is_debug(&self) -> bool {
		self.profile == "debug"
	}
}

fn list_c_files_impl(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
	for e in fs::read_dir(dir)? {
		let e = e?;
		let e_path = e.path();
		let e_type = e.file_type()?;
		if e_type.is_file() {
			let ext = e_path.extension().and_then(OsStr::to_str);
			if !matches!(ext, Some("c" | "s")) {
				continue;
			}
			paths.push(e_path);
		} else if e_type.is_dir() {
			list_c_files_impl(&e_path, paths)?;
		}
	}
	Ok(())
}

/// Lists paths to C and assembly files.
pub fn list_c_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
	let mut paths = vec![];
	list_c_files_impl(dir, &mut paths)?;
	Ok(paths)
}
