/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! Implementation of the directory of a process in the procfs.

mod cmdline;
mod cwd;
mod exe;
mod mounts;
mod stat;
mod status;

use crate::{
	file::{
		fs::kernfs::{node::KernFSNode, KernFS},
		perm,
		perm::{Gid, Uid},
		DirEntry, FileType, Mode,
	},
	process::{pid::Pid, Process},
};
use cmdline::Cmdline;
use cwd::Cwd;
use exe::Exe;
use mounts::Mounts;
use stat::Stat;
use status::Status;
use utils::{boxed::Box, collections::hashmap::HashMap, errno::EResult};

/// The directory of a process.
#[derive(Debug)]
pub struct ProcDir {
	/// The PID of the process.
	pid: Pid,
}

impl ProcDir {
	/// Creates a new instance for the process with the given PID `pid`.
	///
	/// The function adds every node to the given kernfs `fs`.
	pub fn new(pid: Pid, fs: &mut KernFS) -> EResult<Self> {
		let mut entries = HashMap::new();

		// TODO Add every nodes
		// TODO On fail, remove previously inserted nodes

		// Create /proc/<pid>/cmdline
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

		// Create /proc/<pid>/cwd
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

		// Create /proc/<pid>/exe
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

		// Create /proc/<pid>/mounts
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

		// Create /proc/<pid>/stat
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

		// Create /proc/<pid>/status
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
	}
}

impl KernFSNode for ProcDir {
	fn get_file_type(&self) -> FileType {
		FileType::Directory
	}

	fn get_mode(&self) -> Mode {
		0o555
	}

	fn get_uid(&self) -> Uid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_euid()
		} else {
			perm::ROOT_UID
		}
	}

	fn get_gid(&self) -> Gid {
		if let Some(proc_mutex) = Process::get_by_pid(self.pid) {
			proc_mutex.lock().access_profile.get_egid()
		} else {
			perm::ROOT_GID
		}
	}
}
