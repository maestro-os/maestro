/// A filesystem is the representation of the file hierarchy on a storage device.

use crate::device::Device;
use crate::device::DeviceHandle;
use crate::errno::Errno;
use crate::errno;
use crate::util::boxed::Box;
use super::File;
use super::INode;
use super::path::Path;

/// Trait representing a filesystem.
pub trait Filesystem {
	/// Returns the name of the filesystem.
	fn get_name(&self) -> &str;

	/// Tells whether the given device has the current filesystem.
	fn detect(&self, io: &mut dyn DeviceHandle) -> bool;

	/// Loads the file at path `path`.
	fn load_file(&mut self, io: &mut dyn DeviceHandle, path: Path) -> Result<File, Errno>;
	/// Reads from the given node `node` into the buffer `buf`.
	fn read_node(&mut self, io: &mut dyn DeviceHandle, node: INode, buf: &mut [u8])
		-> Result<(), Errno>;
	/// Writes to the given node `node` from the buffer `buf`.
	fn write_node(&mut self, io: &mut dyn DeviceHandle, node: INode, buf: &mut [u8])
		-> Result<(), Errno>;

	// TODO
}

// TODO Array to register filesystems

/// Detects the filesystem on the given device `device`.
pub fn detect(_device: &mut Device) -> Result<Box<dyn Filesystem>, Errno> {
	// TODO

	Err(errno::ENOMEM)
}
