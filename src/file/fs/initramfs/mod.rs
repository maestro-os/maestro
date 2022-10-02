//! The initramfs is a tmpfs which stores initialization files. It is loaded when the kernel boots.

mod cpio;

use cpio::CPIOParser;
use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::fs::Filesystem;
use crate::file::fs::tmp::TmpFS;
use crate::file::path::Path;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::io::DummyIO;

/// Loads and mounts the initramsfs.
///
/// `data` is the slice of data representing the initramfs image.
pub fn load(data: &[u8]) -> Result<(), Errno> {
	// TODO Implement gzip decompression?

	let mut fs = TmpFS::new(usize::MAX, false, Path::root())?;
	let mut io = DummyIO {};

	let cpio_parser = CPIOParser::new(data);
	for entry in cpio_parser {
		let hdr = entry.get_hdr();

		// TODO If path, reorder to avoid creating a file before its parent
		let name = String::from(entry.get_filename())?;
		let file_type = hdr.get_type();
		let perm = hdr.get_perms();
		let content = String::from(entry.get_content())?;

		let content = match file_type {
			FileType::Regular => FileContent::Regular,
			FileType::Directory => FileContent::Directory(HashMap::new()),
			FileType::Link => FileContent::Link(content),
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

		let parent_inode = 0; // TODO
		fs.add_file(&mut io, parent_inode, name, hdr.c_uid, hdr.c_gid, perm, content)?;
	}

	// TODO Register the fs so that it can be used
	todo!();
}
