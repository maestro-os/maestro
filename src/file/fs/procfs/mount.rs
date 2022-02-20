//! This module implements a procfs node which allows to get the list of mountpoint.

use crate::errno::Errno;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::ROOT_GID;
use crate::file::ROOT_UID;
use crate::file::Uid;
use crate::file::fs::kernfs::KernFSNode;
use crate::file::inode::INode;
use crate::time::Timestamp;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::string::String;

/// Structure representing the mount node of the procfs.
pub struct ProcFSMount {}

impl ProcFSMount {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {}
	}
}

impl KernFSNode for ProcFSMount {
	fn get_type(&self) -> FileType {
		FileType::Regular
	}

	fn get_mode(&self) -> Mode {
		0o444
	}

	fn set_mode(&mut self, _mode: Mode) {}

	fn get_uid(&self) -> Uid {
		ROOT_UID
	}

	fn set_uid(&mut self, _uid: Uid) {}

	fn get_gid(&self) -> Gid {
		ROOT_GID
	}

	fn set_gid(&mut self, _gid: Gid) {}

	fn get_atime(&self) -> Timestamp {
		0
	}

	fn set_atime(&mut self, _ts: Timestamp) {}

	fn get_ctime(&self) -> Timestamp {
		0
	}

	fn set_ctime(&mut self, _ts: Timestamp) {}

	fn get_mtime(&self) -> Timestamp {
		0
	}

	fn set_mtime(&mut self, _ts: Timestamp) {}

	fn get_entries(&self) -> Result<HashMap<String, Box<dyn INode>>, Errno> {
		Ok(HashMap::new())
	}
}

impl IO for ProcFSMount {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&self, _offset: u64, _buff: &mut [u8]) -> Result<u64, Errno> {
		// TODO
		todo!();
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}
}
