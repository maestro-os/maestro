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

//! This module implements utility functions for files manipulations.

use super::path::{Component, Path, PathBuf};
use crate::file::{perm::AccessProfile, vfs, vfs::ResolutionSettings, File, FileType};
use utils::{errno, errno::EResult};

/// Creates the directories necessary to reach path `path`.
///
/// If relative, the path is taken from the root.
pub fn create_dirs(path: &Path) -> EResult<()> {
	// Path of the parent directory
	let mut p = PathBuf::root();
	for comp in path.components() {
		let Component::Normal(name) = &comp else {
			continue;
		};
		if let Ok(parent_mutex) = vfs::get_file_from_path(&p, &ResolutionSettings::kernel_follow())
		{
			let mut parent = parent_mutex.lock();
			let res = vfs::create_file(
				&mut parent,
				name,
				&AccessProfile::KERNEL,
				File::new(0, 0, FileType::Directory, 0o755),
			);
			match res {
				Err(e) if e.as_int() != errno::EEXIST => return Err(e),
				_ => {}
			}
		}
		p = p.join(comp)?;
	}
	Ok(())
}
