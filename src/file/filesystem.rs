/// A filesystem is the representation of the file hierarchy on a storage device.

use crate::errno::Errno;
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
