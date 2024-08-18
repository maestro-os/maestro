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

//! The initramfs is a tmpfs stored under the form of an archive. It is used as an initialization
//! environment which doesn't require disk accesses.

mod cpio;

use crate::{
	device, file,
	file::{path::Path, perm::AccessProfile, vfs, vfs::ResolutionSettings, FileType, Stat},
};
use cpio::CPIOParser;
use utils::{errno, errno::EResult, ptr::arc::Arc};

// TODO clean this function
/// Updates the current parent used for the unpacking operation.
///
/// Arguments:
/// - `new` is the new parent's path.
/// - `stored` is the current parent. The tuple contains the path and the file.
/// - `retry` tells whether the function is called as a second try.
fn update_parent<'p>(
	new: &'p Path,
	stored: &mut Option<(&'p Path, Arc<vfs::Entry>)>,
	retry: bool,
) -> EResult<()> {
	// Get the parent
	let result = match stored {
		Some((path, parent)) if new.starts_with(*path) => {
			let Some(name) = new.file_name() else {
				return Ok(());
			};
			let name = Path::new(name)?;
			let rs = ResolutionSettings {
				cwd: Some(parent.clone()),
				..ResolutionSettings::kernel_nofollow()
			};
			vfs::get_file_from_path(name, &rs)
		}
		Some(_) | None => vfs::get_file_from_path(new, &ResolutionSettings::kernel_nofollow()),
	};

	match result {
		Ok(file) => {
			*stored = Some((new, file));
			Ok(())
		}
		// If the directory doesn't exist, create recursively
		Err(e) if !retry && e.as_int() == errno::ENOENT => {
			file::util::create_dirs(new)?;
			update_parent(new, stored, true)
		}
		Err(e) => Err(e),
	}
}

// TODO Implement gzip decompression?
// FIXME The function doesn't work if files are not in the right order in the archive
/// Loads the initramsfs at the root of the VFS.
///
/// `data` is the slice of data representing the initramfs image.
pub fn load(data: &[u8]) -> EResult<()> {
	// TODO Use a stack instead?
	// The stored parent directory
	let mut stored_parent: Option<(&Path, Arc<vfs::Entry>)> = None;

	let cpio_parser = CPIOParser::new(data);
	for entry in cpio_parser {
		let hdr = entry.get_hdr();

		let parent_path = Path::new(entry.get_filename())?;
		let Some(name) = parent_path.file_name() else {
			continue;
		};
		// Change the parent directory if necessary
		let update = match &stored_parent {
			Some((path, _)) => path != &parent_path,
			None => true,
		};
		if update {
			update_parent(parent_path, &mut stored_parent, false)?;
		}

		let parent = &stored_parent.as_ref().unwrap().1;
		// Create file
		let create_result = vfs::create_file(
			parent.clone(),
			name,
			&AccessProfile::KERNEL,
			Stat {
				mode: hdr.c_mode as _,
				uid: hdr.c_uid,
				gid: hdr.c_gid,
				dev_major: device::id::major(hdr.c_rdev as _),
				dev_minor: device::id::minor(hdr.c_rdev as _),
				ctime: 0,
				mtime: 0,
				atime: 0,
				..Default::default()
			},
		);
		let file = match create_result {
			Ok(file_mutex) => file_mutex,
			Err(e) if e.as_int() == errno::EEXIST => continue,
			Err(e) => return Err(e),
		};
		match file.get_type()? {
			FileType::Regular | FileType::Link => {
				let content = entry.get_content();
				file.ops.write_content(&file.location, 0, content)?;
			}
			_ => {}
		}
	}
	Ok(())
}
