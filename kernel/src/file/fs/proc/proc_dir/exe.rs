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
		fs::{proc::get_proc_owner, NodeOps},
		vfs, FileLocation, FileType, Stat,
	},
	format_content,
	process::{pid::Pid, Process},
};
use utils::{errno, errno::EResult};

/// The `exe` node.
#[derive(Debug)]
pub struct Exe(Pid);

impl From<Pid> for Exe {
	fn from(pid: Pid) -> Self {
		Self(pid)
	}
}

impl NodeOps for Exe {
	fn get_stat(&self, _loc: &FileLocation) -> EResult<Stat> {
		let (uid, gid) = get_proc_owner(self.0);
		Ok(Stat {
			mode: FileType::Link.to_mode() | 0o444,
			uid,
			gid,
			..Default::default()
		})
	}

	fn read_content(&self, _loc: &FileLocation, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let path = proc
			.mem_space
			.as_ref()
			.map(|mem_space| vfs::Entry::get_path(&mem_space.lock().exe_info.exe))
			.transpose()?
			.unwrap_or_default();
		format_content!(off, buf, "{path}")
	}
}
