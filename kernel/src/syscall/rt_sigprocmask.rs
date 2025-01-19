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
	process::{
		mem_space::copy::{SyscallPtr, SyscallSlice},
		signal::SigSet,
		Process,
	},
	syscall::Args,
};
use core::{cmp::min, ffi::c_int, intrinsics::unlikely};
use utils::{
	errno,
	errno::{EResult, Errno, EINVAL},
	ptr::arc::Arc,
};

/// Performs the union of the given mask with the current mask.
const SIG_BLOCK: i32 = 0;
/// Clears the bit from the current mask that are set in the given mask.
const SIG_UNBLOCK: i32 = 1;
/// Sets the mask with the given one.
const SIG_SETMASK: i32 = 2;

pub fn rt_sigprocmask(
	Args((how, set, oldset, sigsetsize)): Args<(
		c_int,
		SyscallPtr<SigSet>,
		SyscallPtr<SigSet>,
		usize,
	)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Validation
	if unlikely(sigsetsize != size_of::<SigSet>()) {
		return Err(errno!(EINVAL));
	}
	let mut signal_manager = proc.signal.lock();
	// Save old set
	oldset.copy_to_user(&signal_manager.sigmask)?;
	// Apply new set
	if let Some(set) = set.copy_from_user()? {
		match how {
			SIG_BLOCK => signal_manager.sigmask.0 |= set.0,
			SIG_UNBLOCK => signal_manager.sigmask.0 &= !set.0,
			SIG_SETMASK => signal_manager.sigmask.0 = set.0,
			_ => return Err(errno!(EINVAL)),
		}
	}
	Ok(0)
}
