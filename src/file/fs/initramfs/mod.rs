//! The initramfs is a tmpfs stored under the form of an archive. It is used as an initialization
//! environment which doesn't require disk accesses.

mod cpio;

use cpio::CPIOParser;
use crate::device;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::VFS;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file;
use crate::util::FailableClone;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::io::IO;
use crate::util::ptr::SharedPtr;

/// Updates the current parent used for the unpacking operation.
///
/// Arguments:
/// - `vfs` is the VFS.
/// - `new` is the new parent's path.
/// - `stored` is the current parent. The tuple contains the path and the file.
/// - `retry` tells whether the function is called as a second try.
fn update_parent(
	vfs: &mut VFS,
	curr: &Path,
	stored: &mut Option<(Path, SharedPtr<File>)>,
	retry: bool,
) -> Result<(), Errno> {
	// Getting the parent
	let result = match stored {
		Some((path, file)) if new.begins_with(path) => {
			let name = match new.failable_clone()?.pop() {
				Some(name) => name,
				None => return Ok(()),
			};

			let file_guard = file.lock();
			let f = file_guard.get_mut();

			vfs.get_file_from_parent(f, name, file::ROOT_UID, file::ROOT_GID, false)
		}

		Some(_) | None => vfs.get_file_from_path(new, file::ROOT_UID, file::ROOT_GID, false),
	};

	match result {
		Ok(file) => {
			*stored = Some((new.failable_clone()?, file));
		}

		// If the directory doesn't exist, create recursively
		Err(e) if !retry && e.as_int() == errno::ENOENT => {
			file::util::create_dirs(vfs, new)?;
			return update_parent(vfs, new, stored, true);
		}

		Err(e) => return Err(e),
	}

	Ok(())
}

// TODO Implement gzip decompression?
// FIXME The function doesn't work if files are not in the right order in the archive
/// Loads the initramsfs at the root of the VFS.
///
/// `data` is the slice of data representing the initramfs image.
pub fn load(data: &[u8]) -> Result<(), Errno> {
	let vfs_mutex = vfs::get();
	let vfs_guard = vfs_mutex.lock();
	let vfs = vfs_guard.get_mut().as_mut().unwrap();

	// TODO Use a stack instead?
	// The stored parent directory
	let mut stored_parent: Option<(Path, SharedPtr<File>)> = None;

	let cpio_parser = CPIOParser::new(data);
	for entry in cpio_parser {
		let hdr = entry.get_hdr();

		let mut parent_path = Path::from_str(entry.get_filename(), false)?;
		let name = match parent_path.pop() {
			Some(name) => name,
			None => continue,
		};

		let file_type = hdr.get_type();
		let perm = hdr.get_perms();

		let content = match file_type {
			FileType::Regular => FileContent::Regular,
			FileType::Directory => FileContent::Directory(HashMap::new()),
			FileType::Link => FileContent::Link(String::from(entry.get_content())?),
			FileType::Fifo => FileContent::Fifo,
			FileType::Socket => FileContent::Socket,
			FileType::BlockDevice => FileContent::BlockDevice {
				major: device::id::major(hdr.c_rdev as _),
				minor: device::id::minor(hdr.c_rdev as _),
			},
			FileType::CharDevice => FileContent::CharDevice {
				major: device::id::major(hdr.c_rdev as _),
				minor: device::id::minor(hdr.c_rdev as _),
			},
		};

		// Telling whether the parent directory must be changed
		let update = match &stored_parent {
			Some((path, _)) => path != &parent_path,
			None => true,
		};
		// Change the parent directory if necessary
		if update {
			update_parent(vfs, &parent_path, &mut stored_parent, false)?;
		}

		let parent_mutex = &stored_parent.as_ref().unwrap().1;
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get_mut();

		// Creating file
		let create_result = vfs.create_file(
			parent,
			name,
			file::ROOT_UID,
			file::ROOT_GID,
			perm,
			content,
		);
		let file_mutex = match create_result {
			Ok(file_mutex) => file_mutex,
			Err(e) if e.as_int() == errno::EEXIST => continue,
			Err(e) => return Err(e),
		};

		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		file.set_uid(hdr.c_uid);
		file.set_gid(hdr.c_gid);

		// Writing content if the file is a regular file
		if file_type == FileType::Regular {
			let content = entry.get_content();
			file.write(0, content)?;
		}

		file.sync()?;
	}

	Ok(())
}
