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

//! The `getrusage` system call returns the system usage for the current
//! process.

use crate::process::{mem_space::ptr::SyscallPtr, rusage::RUsage, Process};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno};

/// Returns the resource usage of the current process.
const RUSAGE_SELF: i32 = 0;
/// Returns the resource usage of the process's children.
const RUSAGE_CHILDREN: i32 = -1;

#[syscall]
pub fn getrusage(who: c_int, usage: SyscallPtr<RUsage>) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// TODO Check access to `usage`

	let rusage = match who {
		RUSAGE_SELF => proc.get_rusage().clone(),

		RUSAGE_CHILDREN => {
			// TODO Return resources of terminates children
			RUsage::default()
		}

		_ => return Err(errno!(EINVAL)),
	};

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	let usage_val = usage
		.get_mut(&mut mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	*usage_val = rusage;

	Ok(0)
}
