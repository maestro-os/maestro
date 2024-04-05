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
		fs::{kernfs::node::KernFSNode, Filesystem, NodeOps},
		perm,
		perm::{Gid, Uid},
		DirEntry, FileType, INode, Mode,
	},
	process::{pid::Pid, Process},
};
use cmdline::Cmdline;
use cwd::Cwd;
use exe::Exe;
use mounts::Mounts;
use stat::Stat;
use status::Status;
use utils::{errno, errno::EResult, ptr::cow::Cow};

/// The directory of a process.
#[derive(Debug)]
pub struct ProcDir(pub Pid);

impl KernFSNode for ProcDir {
	fn get_file_type(&self) -> FileType {
		FileType::Directory
	}

	fn get_mode(&self) -> Mode {
		0o555
	}

	fn get_uid(&self) -> Uid {
		if let Some(proc_mutex) = Process::get_by_pid(self.0) {
			proc_mutex.lock().access_profile.get_euid()
		} else {
			perm::ROOT_UID
		}
	}

	fn get_gid(&self) -> Gid {
		if let Some(proc_mutex) = Process::get_by_pid(self.0) {
			proc_mutex.lock().access_profile.get_egid()
		} else {
			perm::ROOT_GID
		}
	}
}

impl NodeOps for ProcDir {
	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		_buf: &mut [u8],
	) -> EResult<u64> {
		Err(errno!(EISDIR))
	}

	fn write_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		_buf: &[u8],
	) -> EResult<()> {
		Err(errno!(EISDIR))
	}

	fn entry_by_name<'n>(
		&self,
		inode: INode,
		fs: &dyn Filesystem,
		name: &'n [u8],
	) -> EResult<Option<DirEntry<'n>>> {
		// TODO add a way to use binary search
		let mut off = 0;
		while let Some((e, next_off)) = Self::next_entry(inode, fs, off)? {
			if e.name.as_ref() == name {
				return Ok(e);
			}
			off = next_off;
		}
		Ok(None)
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let entries: &[(&[u8], &dyn KernFSNode)] = &[
			// /proc/<pid>/cmdline
			(b"cmdline", &Cmdline(self.0)),
			// /proc/<pid>/cwd
			(b"cwd", &Cwd(self.0)),
			// /proc/<pid>/exe
			(b"exe", &Exe(self.0)),
			// /proc/<pid>/mounts
			(b"mounts", &Mounts(self.0)),
			// /proc/<pid>/stat
			(b"stat", &Stat(self.0)),
			// /proc/<pid>/status
			(b"status", &Status(self.0)),
		];
		let entry = entries.get(off).map(|(name, node)| {
			(
				DirEntry {
					inode: 0,
					entry_type: node.get_file_type(),
					name: Cow::Borrowed(name),
				},
				off + 1,
			)
		});
		Ok(entry)
	}
}
