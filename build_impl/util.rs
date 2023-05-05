//! This module implements utility functions.

use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

/// Lists paths to C and assembly files.
pub fn list_c_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
	let mut paths = vec![];

	for e in fs::read_dir(dir)? {
		let e = e?;
		let e_path = e.path();
		let e_type = e.file_type()?;

		if e_type.is_file() {
			let ext = e_path.extension().map(|s| s.to_str()).flatten();
			let keep = match ext {
				Some("c" | "s") => true,
				_ => false,
			};
			if !keep {
				continue;
			}

			paths.push(e_path);
		} else if e_type.is_dir() {
			list_c_files(&e_path)?;
		}
	}

	Ok(paths)
}
