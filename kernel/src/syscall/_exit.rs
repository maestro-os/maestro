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

//! The _exit syscall allows to terminate the current process with the given
//! status code.

use super::Args;
use crate::process::{scheduler, Process};
use core::ffi::c_int;
use utils::{errno::EResult, lock::IntMutexGuard};

/// Exits the current process.
///
/// Arguments:
/// - `status` is the exit status.
/// - `thread_group`: if `true`, the function exits the whole process group.
/// - `proc` is the current process.
pub fn do_exit(status: u32, thread_group: bool, mut proc: IntMutexGuard<Process>) -> ! {
	proc.exit(status);
	let _pid = proc.get_pid();
	let _tid = proc.tid;
	drop(proc);
	if thread_group {
		// TODO Iterate on every process of thread group `tid`, except the
		// process with pid `pid`
	}
	scheduler::end_tick();
	// Cannot resume since the process is now a zombie
	unreachable!();
}

pub fn _exit(Args(status): Args<c_int>, proc: IntMutexGuard<Process>) -> EResult<usize> {
	do_exit(status as _, false, proc);
}
