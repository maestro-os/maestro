//! This module implements the `self` symlink, which points to the current process's directory.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file;
use crate::process::Process;
use crate::process::oom;
use crate::time::unit::Timestamp;
use crate::util::IO;
use crate::util::container::string::String;

/// The `self` symlink.
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
		file::ROOT_UID
	}

	fn set_uid(&mut self, _: Uid) {}

	fn get_gid(&self) -> Gid {
		file::ROOT_GID
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

	fn get_content(&self) -> FileContent {
		let pid = if let Some(proc_mutex) = Process::get_current() {
			let proc_guard = proc_mutex.lock();
			let proc = proc_guard.get();

			proc.get_pid()
		} else {
			0
		};

		let pid_string = oom::wrap(|| String::from_number(pid as _));
		FileContent::Link(pid_string)
	}

	fn set_content(&mut self, _: FileContent) {}

}

impl IO for SelfNode {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, _offset: u64, _buff: &mut [u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}
}
