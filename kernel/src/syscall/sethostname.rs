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

//! The `sethostname` syscall sets the hostname of the system.

use crate::{
	limits,
	process::{mem_space::copy::SyscallSlice, Process},
	syscall::Args,
};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{EResult, Errno},
};

pub fn sethostname(Args((name, len)): Args<(SyscallSlice<u8>, usize)>) -> EResult<usize> {
	// Check the size of the hostname is in bounds
	if len > limits::HOST_NAME_MAX {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Checking permission
	if !proc.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}

	let name = name.copy_from_user(len)?.ok_or(errno!(EFAULT))?;

	let mut hostname = crate::HOSTNAME.lock();
	*hostname = name;

	Ok(0)
}
