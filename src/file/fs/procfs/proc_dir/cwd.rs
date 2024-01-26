//! This module implements the `cwd` node, which is a link to the current
//! working directory of the process.

use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::fs::kernfs::content::KernFSContent;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::perm::Gid;
use crate::file::perm::Uid;
use crate::file::FileContent;
use crate::file::Mode;
use crate::process::pid::Pid;
use crate::process::Process;
use crate::util::io::IO;
use crate::util::TryClone;

/// Struture representing the `cwd` node.
pub struct Cwd {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Cwd {
	fn get_mode(&self) -> Mode {
		0o777
	}

	fn get_uid(&self) -> Uid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_euid()
		} else {
			0
		}
	}

	fn get_gid(&self) -> Gid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_egid()
		} else {
			0
		}
	}

	fn get_content(&mut self) -> EResult<KernFSContent<'_>> {
		let content = Process::get_by_pid(self.pid)
			.map(|mutex| {
				let proc = mutex.lock();
				proc.cwd.0.try_clone()
			})
			.transpose()?
			.unwrap_or_default();
		Ok(KernFSContent::Dynamic(FileContent::Link(content)))
	}
}

impl IO for Cwd {
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
