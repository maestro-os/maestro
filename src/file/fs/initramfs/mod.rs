//! The initramfs is a tmpfs stored under the form of an archive. It is used as an initialization
//! environment which doesn't require disk accesses.

mod cpio;

use cpio::CPIOParser;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file;
use crate::util::FailableClone;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::io::IO;
use crate::util::ptr::SharedPtr;

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
		crate::println!("-> {}", parent_path); // TODO rm
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
				major: 0,
				minor: 0,
			}, // TODO
			FileType::CharDevice => FileContent::CharDevice {
				major: 0,
				minor: 0,
			}, // TODO
		};

		// Telling whether the parent directory must be changed
		let update_parent = match &stored_parent {
			Some((path, _)) => path != &parent_path,
			None => true,
		};

		// Change the parent directory if necessary
		if update_parent {
			stored_parent = Some((
				parent_path.failable_clone()?,
				vfs.get_file_from_path(
					&parent_path,
					file::ROOT_UID,
					file::ROOT_GID,
					false
				)?
			));
		}

		let parent_mutex = &stored_parent.as_ref().unwrap().1;
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get_mut();

		// Creating file
		let create_result = vfs.create_file(
			parent,
			name,
			file::ROOT_UID, // TODO Put the entry's id instead
			file::ROOT_GID, // TODO Put the entry's id instead
			perm,
			content
		);
		let file_mutex = match create_result {
			Ok(file_mutex) => file_mutex,
			Err(e) if e.as_int() == errno::EEXIST => continue,
			Err(e) => return Err(e),
		};

		// Writing content if the file is a regular file
		let content = entry.get_content();
		if file_type == FileType::Regular {
			let file_guard = file_mutex.lock();
			let file = file_guard.get_mut();

			file.write(0, content)?;
		}
	}

	Ok(())
}
