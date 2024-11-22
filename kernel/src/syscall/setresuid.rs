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

//! `setresuid` sets the real, effective and saved user ID of the current process.

use crate::{
	file::perm::{AccessProfile, Uid},
	process::Process,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{errno, errno::EResult, ptr::arc::Arc};

pub fn setresuid(
	Args((ruid, euid, suid)): Args<(c_int, c_int, c_int)>,
	ap: AccessProfile,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Validation
	if ruid < -1 || euid < -1 || suid < -1 {
		return Err(errno!(EINVAL));
	}
	if !ap.is_privileged() {
		let allowed = [-1, ap.uid as _, ap.euid as _, ap.suid as _];
		if !allowed.contains(&ruid) || !allowed.contains(&euid) || !allowed.contains(&suid) {
			return Err(errno!(EPERM));
		}
	}
	// Update
	let mut fs = proc.fs.lock();
	fs.access_profile.uid = match ruid {
		-1 => ap.uid,
		i => i as _,
	};
	fs.access_profile.euid = match euid {
		-1 => ap.euid,
		i => i as _,
	};
	fs.access_profile.suid = match suid {
		-1 => ap.suid,
		i => i as _,
	};
	Ok(0)
}
