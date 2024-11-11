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

//! `setreuid` sets the real and effective user ID of the current process.

use crate::{
	file::perm::{AccessProfile, Uid},
	process::Process,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::EResult,
	lock::{IntMutex, IntMutexGuard},
	ptr::arc::Arc,
};

pub fn setreuid(
	Args((ruid, euid)): Args<(c_int, c_int)>,
	ap: AccessProfile,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Validation
	if ruid < -1 || euid < -1 {
		return Err(errno!(EINVAL));
	}
	if !ap.is_privileged() && ![-1, ap.uid as _, ap.euid as _].contains(&ruid)
		|| ![-1, ap.uid as _, ap.euid as _, ap.suid as _].contains(&euid)
	{
		return Err(errno!(EPERM));
	}
	// Update
	let new_ruid = match ruid {
		-1 => ap.uid,
		i => i as _,
	};
	let new_euid = match euid {
		-1 => ap.euid,
		i => i as _,
	};
	proc.access_profile.uid = new_ruid;
	proc.access_profile.euid = new_euid;
	if new_ruid != ap.uid || new_euid != ap.uid {
		proc.access_profile.suid = new_euid;
	}
	Ok(0)
}
