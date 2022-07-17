//! This module implements a procfs node which allows to get the list of mountpoint.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fs::kernfs::accumulator::Accumulator;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::mountpoint;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::util::IO;
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

	fn read(&mut self, offset: u64, buff: &mut [u8]) -> Result<u64, Errno> {
		if buff.is_empty() {
			return Ok(0);
		}

		let guard = mountpoint::MOUNT_POINTS.lock();
		let container = guard.get_mut();

		let mut iter = container.iter();
		let acc = Accumulator::new(|| {
			let (_, mp_mutex) = iter.next()?;
			let mp_guard = mp_mutex.lock();
			let mp = mp_guard.get();

			let source = "TODO"; // TODO
			let fs_type = "TODO"; // TODO
			let flags = "TODO"; // TODO

			Some(crate::format!("{} {} {} {} 0 0", source, mp.get_path(), fs_type, flags))
		});

		Ok(acc.extract(offset as _, buff)? as _)
	}

	fn write(&mut self, _offset: u64, _buff: &[u8]) -> Result<u64, Errno> {
		Err(errno!(EINVAL))
	}
}
