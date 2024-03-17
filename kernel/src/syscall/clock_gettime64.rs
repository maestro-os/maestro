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

//! `clock_gettime64` is like `clock_gettime` but using 64 bits.

use crate::{
	process::{mem_space::ptr::SyscallPtr, Process},
	time::{
		clock,
		unit::{ClockIdT, Timespec},
	},
};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn clock_gettime64(clockid: ClockIdT, tp: SyscallPtr<Timespec>) -> Result<i32, Errno> {
	let curr_time = clock::current_time_struct::<Timespec>(clockid)?;

	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mut mem_space_guard = mem_space.lock();
		let timespec = tp.get_mut(&mut mem_space_guard)?.ok_or(errno!(EFAULT))?;

		*timespec = curr_time;
	}

	Ok(0)
}
