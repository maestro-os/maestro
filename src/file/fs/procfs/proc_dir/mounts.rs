//! This module implements a procfs node which allows to get the list of mountpoint.

use core::cmp::min;
use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::mountpoint;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::util::DisplayableStr;
use crate::util::IO;
use crate::util::container::string::String;
use crate::util::ptr::cow::Cow;

/// Structure representing the mount node of the procfs.
pub struct Mounts {
	/// The PID of the process.
	pub pid: Pid,
}

impl KernFSNode for Mounts {
	fn get_mode(&self) -> Mode {
		0o444
	}

	fn get_uid(&self) -> Uid {
		let proc_mutex = Process::get_by_pid(self.pid).unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		proc.get_euid()
	}

	fn get_gid(&self) -> Gid {
		let proc_mutex = Process::get_by_pid(self.pid).unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		proc.get_egid()
	}

	fn get_content<'a>(&'a self) -> Cow<'a, FileContent> {
		Cow::from(FileContent::Regular)
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
		let guard = mountpoint::MOUNT_POINTS.lock();
		let container = guard.get_mut();
		for (_, mp_mutex) in container.iter() {
			let mp_guard = mp_mutex.lock();
			let mp = mp_guard.get();

			let fs_guard = mp.get_filesystem();
			let fs = fs_guard.get();

			let fs_type = DisplayableStr {
				s: fs.get_name(),
			};
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

		let content_bytes = content.as_bytes();
		let len = min((content_bytes.len() as u64 - offset) as usize, buff.len());
		buff.copy_from_slice(&content_bytes[(offset as usize)..(offset as usize + len)]);

		let eof = (offset + len as u64) >= content_bytes.len() as u64;
		Ok((len as _, eof))
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}
}
