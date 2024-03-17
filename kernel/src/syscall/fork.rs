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

//! The `fork` system call duplicates the whole current process into a new child
//! process. Execution resumes at the same location for both processes but the
//! return value is different to allow differentiation.

use crate::process::{ForkOptions, Process};
use macros::syscall;
use utils::{errno::Errno, ptr::arc::Arc};

#[syscall]
pub fn fork() -> Result<i32, Errno> {
	// The current process
	let curr_mutex = Process::current_assert();
	// A weak pointer to the new process's parent
	let parent = Arc::downgrade(&curr_mutex);

	let mut curr_proc = curr_mutex.lock();

	let new_mutex = curr_proc.fork(parent, ForkOptions::default())?;
	let mut new_proc = new_mutex.lock();

	// Setting registers
	let mut regs = regs.clone();
	// Setting return value to `0`
	regs.eax = 0;
	new_proc.regs = regs;

	Ok(new_proc.pid as _)
}
