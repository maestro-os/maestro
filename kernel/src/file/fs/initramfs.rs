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

use crate::{
	device, file,
	file::{
		File, FileType, O_WRONLY, Stat, vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	memory::user::UserSlice,
};
use utils::{collections::path::Path, cpio::CPIOParser, errno, errno::EResult, ptr::arc::Arc};

/// Updates the current parent used for the unpacking operation.
///
/// Arguments:
/// - `new` is the new parent path
/// - `parent` is the current parent. The tuple contains the path and the file
/// - `retry` tells whether the function is called as a second try
fn update_parent<'p>(
	new: &'p Path,
	parent: &mut (&'p Path, Arc<vfs::Entry>),
	retry: bool,
) -> EResult<()> {
	// Get the parent
	let result = match new.strip_prefix(parent.0) {
		Some(suffix) => {
			let rs = ResolutionSettings {
				cwd: Some(parent.1.clone()),
				..ResolutionSettings::cur_task(false, false)
			};
			vfs::resolve_path(suffix, &rs).map(|r| {
				let Resolved::Found(r) = r else {
					unreachable!()
				};
				r
			})
		}
		None => vfs::get_file_from_path(new, false),
	};
	match result {
		Ok(ent) => {
			*parent = (new, ent);
			Ok(())
		}
		// If the directory does not exist, create recursively
		Err(e) if !retry && e.as_int() == errno::ENOENT => {
			file::util::create_dirs(new)?;
			update_parent(new, parent, true)
		}
		Err(e) => Err(e),
	}
}

/// Loads the initramsfs at the root of the VFS.
///
/// `data` is the slice of data representing the initramfs image.
pub fn load(data: &[u8]) -> EResult<()> {
	// The stored parent directory
	let mut cur_parent: (&Path, Arc<vfs::Entry>) = (Path::root(), vfs::ROOT.clone());
	let cpio_parser = CPIOParser::new(data);
	for entry in cpio_parser {
		let hdr = entry.get_hdr();
		let path = Path::new(entry.get_filename())?;
		let Some(name) = path.file_name() else {
			continue;
		};
		// Change the parent directory if necessary
		let parent_path = match path.parent() {
			Some(p) if p.is_empty() => Path::root(),
			None => Path::root(),
			Some(p) => p,
		};
		update_parent(parent_path, &mut cur_parent, false)?;
		// Create file
		let create_result = vfs::create_file(
			cur_parent.1.clone(),
			name,
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
		if matches!(file.get_type()?, FileType::Regular | FileType::Link) {
			let content = unsafe { UserSlice::from_slice(entry.get_content()) };
			let file = File::open(file, O_WRONLY)?;
			file.ops.write(&file, 0, content)?;
		}
	}
	Ok(())
}
