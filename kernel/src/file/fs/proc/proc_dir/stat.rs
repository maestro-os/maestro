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

//! Implementation of the `stat` file, which allows to retrieve the current
//! status of the process.

use crate::{
	file::{
		fs::{proc::get_proc_owner, Filesystem, NodeOps},
		FileType, INode, Stat,
	},
	format_content,
	process::{pid::Pid, Process},
};
use core::{fmt, fmt::Formatter};
use utils::{collections::string::String, errno, errno::EResult, DisplayableStr};

struct StatDisp<'p>(&'p Process);

impl<'p> fmt::Display for StatDisp<'p> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		let name = self.0.argv.first().map(String::as_bytes).unwrap_or(b"?");
		// FIXME deadlock
		//let vmem_usage = self.0.get_vmem_usage();
		let vmem_usage = 0;
		let esp = self.0.regs.esp;
		let eip = self.0.regs.eip;
		// TODO Fill every fields with process's data
		write!(
			f,
			"{pid} ({name}) {state_char} {ppid} {pgid} {sid} TODO TODO 0 \
0 0 0 0 {user_jiffies} {kernel_jiffies} TODO TODO {priority} {nice} {num_threads} 0 {vmem_usage} \
TODO TODO TODO TODO {esp} {eip} TODO TODO TODO TODO 0 0 0 TODO TODO TODO TODO TODO TODO TODO TODO \
TODO TODO TODO TODO TODO TODO TODO TODO TODO",
			pid = self.0.get_pid(),
			name = DisplayableStr(name),
			state_char = self.0.get_state().as_char(),
			ppid = self.0.get_parent_pid(),
			pgid = self.0.pgid,
			sid = 0,            // TODO
			user_jiffies = 0,   // TODO
			kernel_jiffies = 0, // TODO
			priority = self.0.priority,
			nice = self.0.nice,
			num_threads = 1, // TODO
		)
	}
}

/// The `stat` node of the proc.
#[derive(Debug)]
pub struct StatNode(Pid);

impl From<Pid> for StatNode {
	fn from(pid: Pid) -> Self {
		Self(pid)
	}
}

impl NodeOps for StatNode {
	fn get_stat(&self, _inode: INode, _fs: &dyn Filesystem) -> EResult<Stat> {
		let (uid, gid) = get_proc_owner(self.0);
		Ok(Stat {
			file_type: FileType::Regular,
			mode: 0o444,
			uid,
			gid,
			..Default::default()
		})
	}

	fn read_content(
		&self,
		_inode: INode,
		_fs: &dyn Filesystem,
		off: u64,
		buf: &mut [u8],
	) -> EResult<(u64, bool)> {
		let proc_mutex = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let proc = proc_mutex.lock();
		format_content!(off, buf, "{}", StatDisp(&proc))
	}
}
