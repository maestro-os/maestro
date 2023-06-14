//! This module implements a procfs node which allows to get the list of
//! mountpoint.

use crate::errno::Errno;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::mountpoint;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::process::pid::Pid;
use crate::process::Process;
use crate::util::container::string::String;
use crate::util::io::IO;
use crate::util::ptr::cow::Cow;
use core::cmp::min;

/// Structure representing the mounts node of the procfs.
pub struct Mounts {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Mounts {
	fn get_mode(&self) -> Mode {
		0o444
	}

	fn get_uid(&self) -> Uid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().euid
		} else {
			0
		}
	}

	fn get_gid(&self) -> Gid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().egid
		} else {
			0
		}
	}

	fn get_content(&self) -> Result<Cow<'_, FileContent>, Errno> {
		Ok(FileContent::Regular.into())
	}
}

impl IO for Mounts {
	fn get_size(&self) -> u64 {
		0
	}

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<(u64, bool), Errno> {
		if buff.is_empty() {
			return Ok((0, false));
		}

		// Generating content
		let mut content = String::new();
		let container = mountpoint::MOUNT_POINTS.lock();

		for (_, mp_mutex) in container.iter() {
			let mp = mp_mutex.lock();

			let fs_type = mp.get_filesystem_type();
			let flags = "TODO"; // TODO

			let s = crate::format!(
				"{} {} {} {} 0 0\n",
				mp.get_source(),
				mp.get_path(),
				fs_type,
				flags
			)?;
			content.append(s)?;
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
