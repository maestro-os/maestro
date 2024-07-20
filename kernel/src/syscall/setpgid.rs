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

//! The `setpgid` system call allows to set the process group ID of a process.

use crate::{
	process::{pid::Pid, Process},
	syscall::Args,
};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::{IntMutex, IntMutexGuard},
	ptr::arc::Arc,
};

pub fn setpgid(
	Args((mut pid, mut pgid)): Args<(Pid, Pid)>,
	proc: Arc<IntMutex<Process>>,
) -> EResult<usize> {
	let mut proc = proc.lock();
	// TODO Check processes SID
	if pid == 0 {
		pid = proc.get_pid();
	}
	if pgid == 0 {
		pgid = pid;
	}
	if pid == proc.get_pid() {
		proc.pgid = pgid;
	} else {
		// Avoid deadlock
		drop(proc);
		Process::get_by_pid(pid)
			.ok_or_else(|| errno!(ESRCH))?
			.lock()
			.set_pgid(pgid)?;
	}
	Ok(0)
}
