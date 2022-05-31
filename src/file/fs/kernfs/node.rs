//! This module implements kernfs nodes.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::time::unit::Timestamp;
use crate::time;
use crate::util::IO;
use crate::util::boxed::Box;

/// Trait representing a node in a kernfs.
pub struct KernFSNode {
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

impl KernFSNode {
	/// Creates a new node.
	/// `mode` is the node's mode.
	/// `uid` is the node owner's user ID.
	/// `gid` is the node owner's group ID.
	/// `content` is the node's content.
	/// `io_handle` is the handle for the node's I/O operations.
	pub fn new(mode: Mode, uid: Uid, gid: Gid, content: FileContent,
		io_handle: Option<Box<dyn IO>>) -> Self {
		// The current timestamp
		let ts = time::get().unwrap_or(0);

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

	/// Returns the number of hard links to the node.
	pub fn get_hard_links_count(&self) -> u16 {
		self.hard_links_count
	}

	/// Sets the number of hard links to the node.
	pub fn set_hard_links_count(&mut self, hard_links_count: u16) {
		self.hard_links_count = hard_links_count;
	}

	/// Returns the permissions of the file.
	pub fn get_mode(&self) -> Mode {
		self.mode
	}

	/// Sets the permissions of the file.
	pub fn set_mode(&mut self, mode: Mode) {
		self.mode = mode;
	}

	/// Returns the UID of the file's owner.
	pub fn get_uid(&self) -> Uid {
		self.uid
	}

	/// Sets the UID of the file's owner.
	pub fn set_uid(&mut self, uid: Uid) {
		self.uid = uid;
	}

	/// Returns the GID of the file's owner.
	pub fn get_gid(&self) -> Gid {
		self.gid
	}

	/// Sets the GID of the file's owner.
	pub fn set_gid(&mut self, gid: Gid) {
		self.gid = gid;
	}

	/// Returns the timestamp of the last access to the file.
	pub fn get_atime(&self) -> Timestamp {
		self.atime
	}

	/// Sets the timestamp of the last access to the file.
	pub fn set_atime(&mut self, ts: Timestamp) {
		self.atime = ts;
	}

	/// Returns the timestamp of the last modification of the file's metadata.
	pub fn get_ctime(&self) -> Timestamp {
		self.ctime
	}

	/// Sets the timestamp of the last modification of the file's metadata.
	pub fn set_ctime(&mut self, ts: Timestamp) {
		self.ctime = ts;
	}

	/// Returns the timestamp of the last modification of the file's content.
	pub fn get_mtime(&self) -> Timestamp {
		self.mtime
	}

	/// Sets the timestamp of the last modification of the file's content.
	pub fn set_mtime(&mut self, ts: Timestamp) {
		self.mtime = ts;
	}

	/// Returns an immutable reference the node's content.
	pub fn get_content(&self) -> &FileContent {
		&self.content
	}

	/// Returns a mutable reference to the node's content.
	pub fn get_content_mut(&mut self) -> &mut FileContent {
		&mut self.content
	}
}

impl IO for KernFSNode {
	fn get_size(&self) -> u64 {
		match &self.io_handle {
			Some(io_handle) => io_handle.get_size(),
			None => 0,
		}
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<u64, Errno> {
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
