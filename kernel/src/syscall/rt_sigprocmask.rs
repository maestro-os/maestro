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

use crate::process::{mem_space::ptr::SyscallSlice, Process};
use core::{cmp::min, ffi::c_int};
use macros::syscall;
use utils::{errno, errno::Errno};

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

// TODO Use SigSet in crate::process::signal
#[syscall]
pub fn rt_sigprocmask(
	how: c_int,
	set: SyscallSlice<u8>,
	oldset: SyscallSlice<u8>,
	sigsetsize: usize,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap().clone();
	let mut mem_space_guard = mem_space.lock();

	let curr = proc.sigmask.as_slice_mut();

	let oldset_slice = oldset.get_mut(&mut mem_space_guard, sigsetsize as _)?;
	if let Some(oldset) = oldset_slice {
		// Save old set
		let len = min(oldset.len(), curr.len());
		oldset[..len].copy_from_slice(&curr[..len]);
	}

	let set_slice = set.get(&mem_space_guard, sigsetsize as _)?;
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
