//! This module implements kernfs nodes.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file;
use crate::time::unit::Timestamp;
use crate::time::unit::TimestampScale;
use crate::time;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::ptr::cow::Cow;

/// Trait representing a node in a kernfs.
pub trait KernFSNode: IO {
	/// Returns the number of hard links to the node.
	fn get_hard_links_count(&self) -> u16 {
		1
	}

	/// Sets the number of hard links to the node.
	fn set_hard_links_count(&mut self, _hard_links_count: u16) {}

	/// Returns the permissions of the file.
	fn get_mode(&self) -> Mode {
		0o777
	}

	/// Sets the permissions of the file.
	fn set_mode(&mut self, _mode: Mode) {}

	/// Returns the UID of the file's owner.
	fn get_uid(&self) -> Uid {
		file::ROOT_UID
	}

	/// Sets the UID of the file's owner.
	fn set_uid(&mut self, _uid: Uid) {}

	/// Returns the GID of the file's owner.
	fn get_gid(&self) -> Gid {
		file::ROOT_GID
	}

	/// Sets the GID of the file's owner.
	fn set_gid(&mut self, _gid: Gid) {}

	/// Returns the timestamp of the last access to the file.
	fn get_atime(&self) -> Timestamp {
		0
	}

	/// Sets the timestamp of the last access to the file.
	fn set_atime(&mut self, _ts: Timestamp) {}

	/// Returns the timestamp of the last modification of the file's metadata.
	fn get_ctime(&self) -> Timestamp {
		0
	}

	/// Sets the timestamp of the last modification of the file's metadata.
	fn set_ctime(&mut self, _ts: Timestamp) {}

	/// Returns the timestamp of the last modification of the file's content.
	fn get_mtime(&self) -> Timestamp {
		0
	}

	/// Sets the timestamp of the last modification of the file's content.
	fn set_mtime(&mut self, _ts: Timestamp) {}

	/// Returns the node's content.
	fn get_content<'a>(&'a self) -> Cow<'a, FileContent>;

	/// Sets the node's content.
	fn set_content(&mut self, _content: FileContent) {}
}

/// Structure representing a dummy kernfs node (with the default behaviour).
pub struct DummyKernFSNode {
	/// The number of hard links to the node.
	hard_links_count: u16,

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

	/// The node's content.
	content: FileContent,

	/// The I/O handle for the node.
	io_handle: Option<Box<dyn IO>>,
}

impl DummyKernFSNode {
	/// Creates a new node.
	/// `mode` is the node's mode.
	/// `uid` is the node owner's user ID.
	/// `gid` is the node owner's group ID.
	/// `content` is the node's content.
	/// `io_handle` is the handle for the node's I/O operations.
	pub fn new(mode: Mode, uid: Uid, gid: Gid, content: FileContent,
		io_handle: Option<Box<dyn IO>>) -> Self {
		// The current timestamp
		let ts = time::get(TimestampScale::Second).unwrap_or(0);

		Self {
			hard_links_count: 1,

			mode,
			uid,
			gid,

			ctime: ts,
			mtime: ts,
			atime: ts,

			content,

			io_handle,
		}
	}
}

impl KernFSNode for DummyKernFSNode {
	fn get_hard_links_count(&self) -> u16 {
		self.hard_links_count
	}

	fn set_hard_links_count(&mut self, hard_links_count: u16) {
		self.hard_links_count = hard_links_count;
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

	fn get_content<'a>(&'a self) -> Cow<'a, FileContent> {
		Cow::from(&self.content)
	}

	fn set_content(&mut self, content: FileContent) {
		self.content = content;
	}
}

impl IO for DummyKernFSNode {
	fn get_size(&self) -> u64 {
		match &self.io_handle {
			Some(io_handle) => io_handle.get_size(),
			None => 0,
		}
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if let Some(io_handle) = &mut self.io_handle {
			return io_handle.read(offset, buff);
		}

		unreachable!();
	}

	fn write(&mut self, offset: u64, buff: &[u8]) -> Result<u64, Errno> {
		if let Some(io_handle) = &mut self.io_handle {
			return io_handle.write(offset, buff);
		}

		unreachable!();
	}
}
