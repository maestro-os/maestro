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

//! Implementation of the `cwd` node, which is a link to the current
//! working directory of the process.

use crate::{
	file::{
		fs::{kernfs::node::KernFSNode, Filesystem, NodeOps},
		perm::{Gid, Uid},
		DirEntry, FileType, INode, Mode,
	},
	format_content,
	process::{pid::Pid, Process},
};
use utils::{errno, errno::EResult, DisplayableStr};

/// The `cwd` node.
#[derive(Debug)]
pub struct Cwd(pub Pid);

impl KernFSNode for Cwd {
	fn get_mode(&self) -> Mode {
		0o777
	}

	fn get_file_type(&self) -> FileType {
		FileType::Link
	}

	fn get_uid(&self) -> Uid {
		if let Some(proc_mutex) = Process::get_by_pid(self.0) {
			proc_mutex.lock().access_profile.get_euid()
		} else {
			0
		}
	}

	fn get_gid(&self) -> Gid {
		if let Some(proc_mutex) = Process::get_by_pid(self.0) {
			proc_mutex.lock().access_profile.get_egid()
		} else {
			0
		}
	}
}

impl NodeOps for Cwd {
	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		let proc_mutex = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let proc = proc_mutex.lock();
		format_content!(off, buf, "{}", DisplayableStr(proc.cwd.0.as_bytes()))
	}

	fn write_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
		_buf: &[u8],
	) -> EResult<u64> {
		Err(errno!(EACCES))
	}

	fn entry_by_name<'n>(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_name: &'n [u8],
	) -> EResult<Option<DirEntry<'n>>> {
		Err(errno!(ENOTDIR))
	}

	fn next_entry(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		_off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		Err(errno!(ENOTDIR))
	}
}
