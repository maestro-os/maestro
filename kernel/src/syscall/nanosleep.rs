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

//! The `nanosleep` system call allows to make the current process sleep for a
//! given delay.

use crate::{
	process::{mem_space::ptr::SyscallPtr, Process},
	time::{clock, clock::CLOCK_MONOTONIC, unit::Timespec32},
};
use macros::syscall;
use utils::{errno, errno::Errno};

// TODO Handle signal interruption (EINTR)

#[syscall]
pub fn nanosleep(req: SyscallPtr<Timespec32>, rem: SyscallPtr<Timespec32>) -> Result<i32, Errno> {
	let start_time = clock::current_time_struct::<Timespec32>(CLOCK_MONOTONIC)?;

	let delay = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		*req.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?
	};

	// Looping until time is elapsed or the process is interrupted by a signal
	loop {
		let curr_time = clock::current_time_struct::<Timespec32>(CLOCK_MONOTONIC)?;
		if curr_time >= start_time + delay {
			break;
		}

		// TODO Allow interruption by signal
		// TODO Make the current process sleep
	}

	// Setting remaining time to zero
	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mut mem_space_guard = mem_space.lock();

		let remaining = rem
			.get_mut(&mut mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		*remaining = Timespec32::default();
	}

	Ok(0)
}
