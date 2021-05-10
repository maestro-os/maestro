/// A filesystem is the representation of the file hierarchy on a storage device.

use crate::device::DeviceType;
use crate::errno;
use crate::errno::Errno;
use crate::util::boxed::Box;
use super::File;
use super::INode;
use super::path::Path;

/// Trait representing a filesystem.
pub trait Filesystem {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &str;

	/// Loads the file at path `path`.
	fn load_file(&mut self, path: Path) -> Result<File, Errno>;
	/// Reads from the given node `node` into the buffer `buf`.
	fn read_node(&mut self, node: INode, buf: &mut [u8]) -> Result<(), Errno>;
	/// Writes to the given node `node` from the buffer `buf`.
	fn write_node(&mut self, node: INode, buf: &mut [u8]) -> Result<(), Errno>;

	// TODO
}

/// Detects the filesystem on the given device.
/// `device_type` is the type of the device.
/// `major` is the major number of the device.
/// `minor` is the minor number of the device.
pub fn detect(_device_type: DeviceType, _major: u32, _minor: u32)
	-> Result<Box<dyn Filesystem>, Errno> {
	// TODO

	Err(errno::ENOMEM)
}
