//! This module implements kernfs directories.

use crate::errno::Errno;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::INode;
use crate::file::Mode;
use crate::file::Uid;
use crate::time::Timestamp;
use crate::util::IO;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;
use crate::util::ptr::SharedPtr;
use super::node::KernFSNode;

/// Structure representing a directory node in a kernfs.
pub struct KernFSDir {
	/// The directory's permissions.
	mode: Mode,
	/// The directory's owner user ID.
	uid: Uid,
	/// The directory's owner group ID.
	gid: Gid,

	/// Timestamp of the last modification of the metadata.
	ctime: Timestamp,
	/// Timestamp of the last modification of the file.
	mtime: Timestamp,
	/// Timestamp of the last access to the file.
	atime: Timestamp,

	/// The directory's entries.
	entries: HashMap<String, (INode, SharedPtr<dyn KernFSNode>)>,
}

impl KernFSNode for KernFSDir {
	fn get_type(&self) -> FileType {
		FileType::Directory
	}

	fn get_mode(&self) -> Mode {
		self.mode
	}

	fn set_mode(&mut self, mode: Mode) {
		self.mode = mode;
	}

	fn get_uid(&self) -> Uid {
		self.uid
	}

	fn set_uid(&mut self, uid: Uid) {
		self.uid = uid;
	}

	fn get_gid(&self) -> Gid {
		self.gid
	}

	fn set_gid(&mut self, gid: Gid) {
		self.gid = gid;
	}

	fn get_atime(&self) -> Timestamp {
		self.atime
	}

	fn set_atime(&mut self, ts: Timestamp) {
		self.atime = ts;
	}

	fn get_ctime(&self) -> Timestamp {
		self.ctime
	}

	fn set_ctime(&mut self, ts: Timestamp) {
		self.ctime = ts;
	}

	fn get_mtime(&self) -> Timestamp {
		self.mtime
	}

	fn set_mtime(&mut self, ts: Timestamp) {
		self.mtime = ts;
	}

	fn get_entries(&self) -> &HashMap<String, (INode, SharedPtr<dyn KernFSNode>)> {
		&self.entries
	}
}

impl IO for KernFSDir {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<u64, Errno> {
		Err(errno!(EISDIR))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EISDIR))
	}
}
