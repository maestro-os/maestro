//! This module implements kernfs nodes.

use crate::errno::Errno;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::time::unit::Timestamp;
use crate::util::IO;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::ptr::SharedPtr;

/// Trait representing a node in a kernfs.
pub trait KernFSNode: IO {
	/// Returns the type of the node.
	fn get_type(&self) -> FileType;

	/// Returns the permissions of the file.
	fn get_mode(&self) -> Mode;
	/// Sets the permissions of the file.
	fn set_mode(&mut self, mode: Mode);

	/// Returns the UID of the file's owner.
	fn get_uid(&self) -> Uid;
	/// Sets the UID of the file's owner.
	fn set_uid(&mut self, uid: Uid);
	/// Returns the GID of the file's owner.
	fn get_gid(&self) -> Gid;
	/// Sets the GID of the file's owner.
	fn set_gid(&mut self, gid: Gid);

	/// Returns the timestamp of the last access to the file.
	fn get_atime(&self) -> Timestamp;
	/// Sets the timestamp of the last access to the file.
	fn set_atime(&mut self, ts: Timestamp);

	/// Returns the timestamp of the last modification of the file's metadata.
	fn get_ctime(&self) -> Timestamp;
	/// Sets the timestamp of the last modification of the file's metadata.
	fn set_ctime(&mut self, ts: Timestamp);

	/// Returns the timestamp of the last modification of the file's content.
	fn get_mtime(&self) -> Timestamp;
	/// Sets the timestamp of the last modification of the file's content.
	fn set_mtime(&mut self, ts: Timestamp);

	/// Returns the list of entries in the node.
	fn get_entries(&self) -> &HashMap<String, (INode, SharedPtr<dyn KernFSNode>)>;

	/// Returns the inode of the entry with name `name`.
	/// If the entry doesn't exist, the function returns an error.
	/// Reimplementing this function is highly recommended since the default implementation is
	/// using get_entries, which can be fairly slow depending on the amount of elements in the
	/// node.
	fn get_entry(&self, name: &String) -> Result<(INode, SharedPtr<dyn KernFSNode>), Errno> {
		Ok(self.get_entries().get(name).ok_or_else(|| errno!(ENOENT))?.clone())
	}
}
