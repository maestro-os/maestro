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
		fs::{proc::get_proc_owner, FileOps},
		vfs, File, FileType, Stat,
	},
	format_content,
	process::{pid::Pid, Process},
};
use utils::{errno, errno::EResult};

/// The `cwd` node.
#[derive(Debug)]
pub struct Cwd(Pid);

impl From<Pid> for Cwd {
	fn from(pid: Pid) -> Self {
		Self(pid)
	}
}

impl FileOps for Cwd {
	fn get_stat(&self, _file: &File) -> EResult<Stat> {
		let (uid, gid) = get_proc_owner(self.0);
		Ok(Stat {
			mode: FileType::Link.to_mode() | 0o444,
			uid,
			gid,
			..Default::default()
		})
	}

	fn read(&self, _file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let fs = proc.fs.lock();
		let cwd = vfs::Entry::get_path(&fs.cwd)?;
		format_content!(off, buf, "{cwd}")
	}
}
