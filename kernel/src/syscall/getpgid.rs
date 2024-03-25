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

//! This module implements the `getpgid` system call, which allows to get the
//! process group ID of a process.

use crate::process::{pid::Pid, Process};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn getpgid(pid: Pid) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	if pid == 0 {
		Ok(proc.pgid as _)
	} else {
		let proc_mutex = {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno!(ESRCH));
			}
		};
		let proc = proc_mutex.lock();

		Ok(proc.pgid as _)
	}
}
