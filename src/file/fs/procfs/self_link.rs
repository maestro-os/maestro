//! This module implements the `self` symlink, which points to the current
//! process's directory.

use crate::{
	errno::{EResult, Errno},
	file::{
		fs::kernfs::{content::KernFSContent, node::KernFSNode},
		path::PathBuf,
		perm,
		perm::{Gid, Uid},
		FileContent, Mode,
	},
	process::Process,
	time::unit::Timestamp,
	util::io::IO,
};

/// The `self` symlink.
#[derive(Debug)]
pub struct SelfNode {}

impl KernFSNode for SelfNode {
	fn get_hard_links_count(&self) -> u16 {
		1
	}

	fn set_hard_links_count(&mut self, _: u16) {}

	fn get_mode(&self) -> Mode {
		0o777
	}

	fn set_mode(&mut self, _: Mode) {}

	fn get_uid(&self) -> Uid {
		perm::ROOT_UID
	}

	fn set_uid(&mut self, _: Uid) {}

	fn get_gid(&self) -> Gid {
		perm::ROOT_GID
	}

	fn set_gid(&mut self, _: Gid) {}

	fn get_atime(&self) -> Timestamp {
		0
	}

	fn set_atime(&mut self, _: Timestamp) {}

	fn get_ctime(&self) -> Timestamp {
		0
	}

	fn set_ctime(&mut self, _: Timestamp) {}

	fn get_mtime(&self) -> Timestamp {
		0
	}

	fn set_mtime(&mut self, _: Timestamp) {}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		let pid = Process::current_assert().lock().pid;
		let pid = PathBuf::try_from(crate::format!("{pid}")?)?;
		Ok(FileContent::Link(pid).into())
	}
}

impl IO for SelfNode {
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
