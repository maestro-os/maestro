//! This module implements the directory of a process in the procfs.

mod cmdline;
mod cwd;
mod exe;
mod mounts;
mod stat;
mod status;

use crate::errno::AllocError;
use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::fs::kernfs::content::KernFSContent;
use crate::file::fs::kernfs::node::KernFSNode;
use crate::file::fs::kernfs::KernFS;
use crate::file::perm::Gid;
use crate::file::perm::Uid;
use crate::file::DirEntry;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Mode;
use crate::process::oom;
use crate::process::pid::Pid;
use crate::process::Process;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::io::IO;
use cmdline::Cmdline;
use cwd::Cwd;
use exe::Exe;
use mounts::Mounts;
use stat::Stat;
use status::Status;

/// Structure representing the directory of a process.
pub struct ProcDir {
	/// The PID of the process.
	pid: Pid,

	/// The content of the directory. This will always be a Directory variant.
	content: FileContent,
}

impl ProcDir {
	/// Creates a new instance for the process with the given PID `pid`.
	///
	/// The function adds every nodes to the given kernfs `fs`.
	pub fn new(pid: Pid, fs: &mut KernFS) -> Result<Self, Errno> {
		let mut entries = HashMap::new();

		// TODO Add every nodes
		// TODO On fail, remove previously inserted nodes

		// Creating /proc/<pid>/cmdline
		let node = Cmdline {
			pid,
		};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"cmdline".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		// Creating /proc/<pid>/cwd
		let node = Cwd {
			pid,
		};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"cwd".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Link,
			},
		)?;

		// Creating /proc/<pid>/exe
		let node = Exe {
			pid,
		};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"exe".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Link,
			},
		)?;

		// Creating /proc/<pid>/mounts
		let node = Mounts {
			pid,
		};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"mounts".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		// Creating /proc/<pid>/stat
		let node = Stat {
			pid,
		};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"stat".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		// Creating /proc/<pid>/status
		let node = Status {
			pid,
		};
		let inode = fs.add_node(Box::new(node)?)?;
		entries.insert(
			b"status".try_into()?,
			DirEntry {
				inode,
				entry_type: FileType::Regular,
			},
		)?;

		Ok(Self {
			pid,

			content: FileContent::Directory(entries),
		})
	}

	/// Removes inner nodes in order to drop the current node.
	///
	/// If this function isn't called, the the kernel will be leaking the nodes
	/// (which is bad).
	///
	/// `fs` is the procfs.
	pub fn drop_inner(&mut self, fs: &mut KernFS) {
		match &mut self.content {
			FileContent::Directory(entries) => {
				for (_, entry) in entries.iter() {
					oom::wrap(|| fs.remove_node(entry.inode).map_err(|_| AllocError));
				}

				entries.clear();
			}

			_ => unreachable!(),
		}
	}
}

impl KernFSNode for ProcDir {
	fn get_mode(&self) -> Mode {
		0o555
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
		Ok(KernFSContent::Owned(&mut self.content))
	}
}

impl IO for ProcDir {
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

impl Drop for ProcDir {
	fn drop(&mut self) {
		// Making sure inner nodes have been dropped
		match &self.content {
			FileContent::Directory(entries) => debug_assert!(entries.is_empty()),
			_ => unreachable!(),
		}
	}
}
