//! The cmdline node allows to retrieve the list of command line arguments of
//! the process.

use crate::{
	errno::{EResult, Errno},
	file::{
		fs::kernfs::{content::KernFSContent, node::KernFSNode},
		perm::{Gid, Uid},
		FileContent, Mode,
	},
	process::{pid::Pid, Process},
	util::{container::string::String, io::IO},
};
use core::cmp::min;

/// Structure representing the cmdline node of the procfs.
#[derive(Debug)]
pub struct Cmdline {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Cmdline {
	fn get_mode(&self) -> Mode {
		0o444
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
		Ok(KernFSContent::Dynamic(FileContent::Regular))
	}
}

impl IO for Cmdline {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if buff.is_empty() {
			return Ok((0, false));
		}

		let proc_mutex = Process::get_by_pid(self.pid).ok_or_else(|| errno!(ENOENT))?;
		let proc = proc_mutex.lock();

		// Generating content
		let mut content = String::new();
		for a in proc.argv.iter() {
			content.push_str(a)?;
			content.push(b'\0')?;
		}

		// Copying content to userspace buffer
		let content_bytes = content.as_bytes();
		let len = min((content_bytes.len() as u64 - offset) as usize, buff.len());
		buff[..len].copy_from_slice(&content_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= content_bytes.len() as u64;
		Ok((len as _, eof))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}

	fn poll(&mut self, _mask: u32) -> Result<u32, Errno> {
		// TODO
		todo!();
	}
}
