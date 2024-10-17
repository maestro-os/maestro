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

//! Build script utilities

use std::{
	ffi::OsStr,
	fs, io,
	path::{Path, PathBuf},
};

fn list_c_files_impl(dir: &Path, paths: &mut Vec<PathBuf>) -> io::Result<()> {
	let dir = match fs::read_dir(dir) {
		Ok(dir) => dir,
		Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
		Err(e) => return Err(e),
	};
	for e in dir {
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
