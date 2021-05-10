//! The ext2 filesystem is a classical filesystem used in Unix systems.
//! It is nowdays obsolete and has been replaced by ext3 and ext4.

use crate::device::DeviceHandle;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::INode;
use crate::file::filesystem::Filesystem;
use crate::file::filesystem::FilesystemType;
use crate::file::path::Path;
use crate::util::boxed::Box;

/// Structure representing the ext2 filesystem type.
pub struct Ext2FsType {}

impl FilesystemType for Ext2FsType {
	fn get_name(&self) -> &str {
		"ext2"
	}

	fn detect(&self, _io: &mut dyn DeviceHandle) -> bool {
		// TODO

		false
	}

	fn new_filesystem(&self, _io: &mut dyn DeviceHandle) -> Result<Box<dyn Filesystem>, Errno> {
		// TODO
		Err(errno::ENOMEM)
	}
}

/// Structure representing a instance of the ext2 filesystem.
pub struct Ext2Fs {}

impl Ext2Fs {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {}
	}
}

impl Filesystem for Ext2Fs {
	fn get_name(&self) -> &str {
		"ext2"
	}

	fn load_file(&mut self, _io: &mut dyn DeviceHandle, _path: Path) -> Result<File, Errno> {
		// TODO

		Err(errno::ENOMEM)
	}

	fn read_node(&mut self, _io: &mut dyn DeviceHandle, _node: INode, _buf: &mut [u8])
		-> Result<(), Errno> {
		// TODO

		Err(errno::ENOMEM)
	}

	fn write_node(&mut self, _io: &mut dyn DeviceHandle, _node: INode, _buf: &mut [u8])
		-> Result<(), Errno> {
		// TODO

		Err(errno::ENOMEM)
	}
}
