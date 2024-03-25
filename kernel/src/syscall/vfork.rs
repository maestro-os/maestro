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

//! The `vfork` system call works the same as the `fork` system call, except the
//! parnet process is blocked until the child process exits or executes a
//! program. During that time, the child process also shares the same memory
//! space as the parent.

use crate::process::{scheduler, ForkOptions, Process};
use macros::syscall;
use utils::{errno::Errno, ptr::arc::Arc};

#[syscall]
pub fn vfork() -> Result<i32, Errno> {
	let new_pid = {
		// The current process
		let curr_mutex = Process::current_assert();
		// A weak pointer to the new process's parent
		let parent = Arc::downgrade(&curr_mutex);

		let mut curr_proc = curr_mutex.lock();

		let fork_options = ForkOptions {
			vfork: true,
			..ForkOptions::default()
		};
		let new_mutex = curr_proc.fork(parent, fork_options)?;
		let mut new_proc = new_mutex.lock();

		// Setting registers
		let mut regs = regs.clone();
		// Setting return value to `0`
		regs.eax = 0;
		new_proc.regs = regs;

		new_proc.pid
	};

	// Letting another process run instead of the current. Because the current
	// process must now wait for the child process to terminate or execute a program
	scheduler::end_tick();

	Ok(new_pid as _)
}
