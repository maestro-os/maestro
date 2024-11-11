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

use crate::{
	process::{mem_space::copy::SyscallPtr, rusage::RUsage, Process},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// Returns the resource usage of the current process.
const RUSAGE_SELF: i32 = 0;
/// Returns the resource usage of the process's children.
const RUSAGE_CHILDREN: i32 = -1;

pub fn getrusage(Args((who, usage)): Args<(c_int, SyscallPtr<RUsage>)>) -> EResult<usize> {
	let rusage = match who {
		RUSAGE_SELF => Process::current().rusage.clone(),
		RUSAGE_CHILDREN => {
			// TODO Return resources of terminated children
			RUsage::default()
		}
		_ => return Err(errno!(EINVAL)),
	};
	usage.copy_to_user(rusage)?;
	Ok(0)
}
