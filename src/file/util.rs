//! This module implements utility functions for files manipulations.

use super::path::Path;
use super::vfs::VFS;
use super::File;
use super::FileContent;
use crate::errno;
use crate::errno::Errno;
use crate::file::Gid;
use crate::file::Uid;
use crate::memory;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::io::IO;
use crate::util::FailableClone;

/// Creates the directories necessary to reach path `path`. On success, the
/// function returns the number of created directories (without the directories
/// that already existed). If relative, the path is taken from the root.
/// `vfs` is a reference to the VFS.
pub fn create_dirs(vfs: &mut VFS, path: &Path) -> Result<usize, Errno> {
	let path = Path::root().concat(path)?;

	// The path of the parent directory
	let mut p = Path::root();
	// The number of created directories
	let mut created_count = 0;

	for i in 0..path.get_elements_count() {
		let name = path[i].failable_clone()?;

		if let Ok(parent_mutex) = vfs.get_file_from_path(&p, 0, 0, true) {
			let parent_guard = parent_mutex.lock();
			let parent = parent_guard.get_mut();

			match vfs.create_file(
				parent,
				name.failable_clone()?,
				0,
				0,
				0o755,
				FileContent::Directory(HashMap::new()),
			) {
				Ok(_) => created_count += 1,
				Err(e) if e.as_int() != errno::EEXIST => return Err(e),

				_ => {}
			}
		}

		p.push(name)?;
	}

	Ok(created_count)
}

/// Copies the file `old` into the directory `new_parent` with name `new_name`.
/// `vfs` is a reference to the VFS.
pub fn copy_file(
	vfs: &mut VFS,
	old: &mut File,
	new_parent: &mut File,
	new_name: String,
) -> Result<(), Errno> {
	let uid = old.get_uid();
	let gid = old.get_gid();
	let mode = old.get_mode();
	let content = old.get_content();

	match content {
		// Copy the file and its content
		FileContent::Regular => {
			let new_mutex =
				vfs.create_file(new_parent, new_name, uid, gid, mode, FileContent::Regular)?;
			let new_guard = new_mutex.lock();
			let new = new_guard.get_mut();

			// TODO On fail, remove file
			// Copying content
			let mut off = 0;
			let mut buff: [u8; memory::PAGE_SIZE] = [0; memory::PAGE_SIZE];
			loop {
				let (len, eof) = old.read(off, &mut buff)?;
				if eof {
					break;
				}

				new.write(off, &buff)?;
				off += len;
			}
		}

		// Copy the directory recursively
		FileContent::Directory(entries) => {
			let new_mutex = vfs.create_file(
				new_parent,
				new_name,
				uid,
				gid,
				mode,
				FileContent::Directory(HashMap::new()),
			)?;
			let new_guard = new_mutex.lock();
			let new = new_guard.get_mut();

			// TODO On fail, undo
			for (name, _) in entries.iter() {
				let old_mutex =
					vfs.get_file_from_parent(new, name.failable_clone()?, uid, gid, false)?;
				let old_guard = old_mutex.lock();
				let old = old_guard.get_mut();

				copy_file(vfs, old, new, name.failable_clone()?)?;
			}
		}

		// Copy the file
		_ => {
			vfs.create_file(
				new_parent,
				new_name,
				uid,
				gid,
				mode,
				content.failable_clone()?,
			)?;
		}
	}

	Ok(())
}

/// Removes the file `file` and its subfiles recursively if it's a directory.
/// `vfs` is a reference to the VFS.
/// `uid` is the user ID used to check permissions.
/// `gid` is the group ID used to check permissions.
pub fn remove_recursive(vfs: &mut VFS, file: &mut File, uid: Uid, gid: Gid) -> Result<(), Errno> {
	let content = file.get_content().failable_clone()?;

	match content {
		FileContent::Directory(entries) => {
			for (name, _) in entries.iter() {
				let name = name.failable_clone()?;
				let subfile_mutex = vfs.get_file_from_parent(file, name, uid, gid, false)?;
				let subfile_guard = subfile_mutex.lock();
				let subfile = subfile_guard.get_mut();

				remove_recursive(vfs, subfile, uid, gid)?;
			}
		}

		_ => vfs.remove_file(file, uid, gid)?,
	}

	Ok(())
}
