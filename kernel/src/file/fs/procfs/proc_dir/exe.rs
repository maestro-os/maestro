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

//! Implementation of the `exe` node, which is a link to the executable
//! file of the process.

use crate::{
	file::{
		fs::{
			procfs::{get_proc_gid, get_proc_uid},
			Filesystem, NodeOps,
		},
		perm::{Gid, Uid},
		DirEntry, FileType, INode,
	},
	format_content,
	process::{pid::Pid, Process},
};
use utils::{errno, errno::EResult, DisplayableStr};

/// The `exe` node.
#[derive(Debug)]
pub struct Exe(pub Pid);

impl NodeOps for Exe {
	fn get_file_type(&self) -> FileType {
		FileType::Regular
	}

	fn get_uid(&self) -> Uid {
		get_proc_uid(self.0)
	}

	fn get_gid(&self) -> Gid {
		get_proc_gid(self.0)
	}

	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<u64> {
		let proc_mutex = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let proc = proc_mutex.lock();
		format_content!(off, buf, "{}", DisplayableStr(proc.exec_path.as_bytes()))
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
	) -> EResult<Option<(DirEntry<'n>, u64)>> {
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
