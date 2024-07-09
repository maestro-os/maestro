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

//! The rt_sigprocmask system call allows to change the blocked signal mask.

use crate::{
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use core::{cmp::min, ffi::c_int};
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

// TODO Use SigSet in crate::process::signal
pub fn rt_sigprocmask(
	Args((how, set, oldset, sigsetsize)): Args<(c_int, SyscallSlice<u8>, SyscallSlice<u8>, usize)>,
) -> EResult<usize> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	// Save old set
	let curr = proc.sigmask.as_slice_mut();
	let len = min(curr.len(), sigsetsize as _);
	oldset.copy_to_user(&curr[..len])?;

	let set_slice = set.copy_from_user(sigsetsize as _)?;
	if let Some(set) = set_slice {
		// Applies the operation
		match how {
			SIG_BLOCK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] |= set[i];
				}
			}

			SIG_UNBLOCK => {
				for i in 0..min(set.len(), curr.len()) {
					curr[i] &= !set[i];
				}
			}

			SIG_SETMASK => {
				let len = min(set.len(), curr.len());
				curr[..len].copy_from_slice(&set[..len]);
			}

			_ => return Err(errno!(EINVAL)),
		}
	}

	Ok(0)
}
