//! TODO doc

mod osrelease;

use super::kernfs::KernFS;
use crate::{
	errno::{EResult, Errno},
	file::{
		fs::kernfs::{content::KernFSContent, node::KernFSNode},
		perm::{Gid, Uid},
		DirEntry, FileContent, FileType, Mode,
	},
	util::{boxed::Box, collections::hashmap::HashMap, io::IO},
};
use osrelease::OsRelease;

// TODO Handle dropping
/// Structure representing the `kernel` directory.
#[derive(Debug)]
pub struct KernelDir {
	/// The content of the directory. This will always be a Directory variant.
	content: FileContent,
}

impl KernelDir {
	/// Creates a new instance.
	///
	/// The function adds every nodes to the given kernfs `fs`.
	pub fn new(fs: &mut KernFS) -> Result<Self, Errno> {
		let mut entries = HashMap::new();

		// TODO Add every nodes
		// TODO On fail, remove previously inserted nodes

		// Creating /proc/sys/kernel
		let node = OsRelease {};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"osrelease".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		Ok(Self {
			content: FileContent::Directory(entries),
		})
	}
}

impl KernFSNode for KernelDir {
	fn get_mode(&self) -> Mode {
		0o555
	}

	fn get_uid(&self) -> Uid {
		0
	}

	fn get_gid(&self) -> Gid {
		0
	}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		Ok(KernFSContent::Owned(&mut self.content))
	}
}

impl IO for KernelDir {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		Err(errno!(EINVAL))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		Err(errno!(EINVAL))
	}
}
