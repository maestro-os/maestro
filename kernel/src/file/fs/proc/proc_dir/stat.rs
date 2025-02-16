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
		fs::{proc::get_proc_owner, FileOps},
		File, FileType, Stat,
	},
	format_content,
	memory::VirtAddr,
	process::{pid::Pid, Process},
};
use core::fmt;
use utils::{errno, errno::EResult};

/// The `stat` node of the proc.
#[derive(Debug)]
pub struct StatNode(pub Pid);

impl FileOps for StatNode {
	fn get_stat(&self, _file: &File) -> EResult<Stat> {
		let (uid, gid) = get_proc_owner(self.0);
		Ok(Stat {
			mode: FileType::Regular.to_mode() | 0o444,
			uid,
			gid,
			..Default::default()
		})
	}

	fn read(&self, _file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let proc = Process::get_by_pid(self.0).ok_or_else(|| errno!(ENOENT))?;
		let mem_space = proc.mem_space.as_ref().unwrap().lock();
		let disp = fmt::from_fn(|f| {
			let user_regs = proc.user_regs();
			// TODO Fill every fields with process's data
			write!(
				f,
				"{pid} ({name}) {state_char} {ppid} {pgid} {sid} TODO TODO 0 \
0 0 0 0 {user_jiffies} {kernel_jiffies} TODO TODO {priority} {nice} {num_threads} 0 {vmem_usage} \
TODO TODO TODO TODO {sp:?} {pc:?} TODO TODO TODO TODO 0 0 0 TODO TODO TODO TODO TODO TODO TODO TODO \
TODO TODO TODO TODO TODO TODO TODO TODO TODO",
				pid = self.0,
				name = mem_space.exe_info.exe.name,
				state_char = proc.get_state().as_char(),
				ppid = proc.get_parent_pid(),
				pgid = proc.get_pgid(),
				sid = 0,            // TODO
				user_jiffies = 0,   // TODO
				kernel_jiffies = 0, // TODO
				priority = 0, // TODO
				nice = 0, // TODO
				num_threads = 1, // TODO
				vmem_usage = mem_space.get_vmem_usage(),
				sp = VirtAddr(user_regs.get_stack_address() as _),
				pc = VirtAddr(user_regs.get_program_counter() as _),
			)
		});
		format_content!(off, buf, "{disp}")
	}
}
