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

//! `setresgid` sets the real, effective and saved group ID of the current process.

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

pub fn setresgid(
	Args((rgid, egid, sgid)): Args<(c_int, c_int, c_int)>,
	ap: AccessProfile,
	proc: Arc<IntMutex<Process>>,
) -> EResult<usize> {
	// Validation
	if rgid < -1 || egid < -1 || sgid < -1 {
		return Err(errno!(EINVAL));
	}
	if !ap.is_privileged() {
		let allowed = [-1, ap.gid as _, ap.egid as _, ap.sgid as _];
		if !allowed.contains(&rgid) || !allowed.contains(&egid) || !allowed.contains(&sgid) {
			return Err(errno!(EPERM));
		}
	}
	// Update
	let mut proc = proc.lock();
	proc.access_profile.gid = match rgid {
		-1 => ap.gid,
		i => i as _,
	};
	proc.access_profile.egid = match egid {
		-1 => ap.egid,
		i => i as _,
	};
	proc.access_profile.sgid = match sgid {
		-1 => ap.sgid,
		i => i as _,
	};
	Ok(0)
}
