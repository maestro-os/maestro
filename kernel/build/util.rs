//! Build script utilities

use std::{
	ffi::OsStr,
	fs, io,
	path::{Path, PathBuf},
};

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
