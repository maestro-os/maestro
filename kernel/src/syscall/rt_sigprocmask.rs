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

//! The `rt_sigprocmask` system call allows to change the blocked signal mask.

use crate::{
	process::{mem_space::copy::SyscallSlice, signal::SigSet, Process},
	syscall::Args,
};
use core::{cmp::min, ffi::c_int, intrinsics::unlikely};
use utils::{
	errno,
	errno::{EResult, Errno, EINVAL},
	lock::{IntMutex, IntMutexGuard},
	ptr::arc::Arc,
};

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

pub fn rt_sigprocmask(
	Args((how, set, oldset, sigsetsize)): Args<(c_int, SyscallSlice<u8>, SyscallSlice<u8>, usize)>,
	proc: Arc<IntMutex<Process>>,
) -> EResult<usize> {
	// Validation
	if unlikely(sigsetsize != 8) {
		return Err(errno!(EINVAL));
	}
	let mut proc = proc.lock();
	// Save old set
	let cur = proc.sigmask.0.to_ne_bytes();
	oldset.copy_to_user(0, &cur)?;
	// Apply new set
	if let Some(set) = set.copy_from_user(..8)? {
		let set = u64::from_ne_bytes(set.try_into().unwrap());
		match how {
			SIG_BLOCK => proc.sigmask.0 |= set,
			SIG_UNBLOCK => proc.sigmask.0 &= !set,
			SIG_SETMASK => proc.sigmask.0 = set,
			_ => return Err(errno!(EINVAL)),
		}
	}
	Ok(0)
}
