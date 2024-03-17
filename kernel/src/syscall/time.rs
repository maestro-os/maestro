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

//! The `time` syscall allows to retrieve the number of seconds elapsed since
//! the UNIX Epoch.

use crate::{
	process::{mem_space::ptr::SyscallPtr, Process},
	time::{clock, clock::CLOCK_MONOTONIC, unit::TimestampScale},
};
use macros::syscall;
use utils::errno::Errno;

// TODO Watch for timestamp overflow

#[syscall]
pub fn time(tloc: SyscallPtr<u32>) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	// Getting the current timestamp
	let time = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second)?;

	// Writing the timestamp to the given location, if not null
	if let Some(tloc) = tloc.get_mut(&mut mem_space_guard)? {
		*tloc = time as _;
	}

	Ok(time as _)
}
