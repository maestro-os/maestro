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

//! Users and groups system calls.

use crate::{
	file::perm::{AccessProfile, Gid, Uid},
	process::Process,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{errno, errno::EResult, ptr::arc::Arc};

pub fn getuid(ap: AccessProfile) -> EResult<usize> {
	Ok(ap.uid as _)
}

pub fn getgid(ap: AccessProfile) -> EResult<usize> {
	Ok(ap.gid as _)
}

pub fn geteuid(ap: AccessProfile) -> EResult<usize> {
	Ok(ap.euid as _)
}

pub fn getegid(ap: AccessProfile) -> EResult<usize> {
	Ok(ap.egid as _)
}

pub fn setuid(Args(uid): Args<Uid>, proc: Arc<Process>) -> EResult<usize> {
	proc.fs.lock().access_profile.set_uid(uid)?;
	Ok(0)
}

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
	let mut fs = proc.fs.lock();
	fs.access_profile.uid = new_ruid;
	fs.access_profile.euid = new_euid;
	if new_ruid != ap.uid || new_euid != ap.uid {
		fs.access_profile.suid = new_euid;
	}
	Ok(0)
}

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

pub fn setgid(Args(gid): Args<Gid>, proc: Arc<Process>) -> EResult<usize> {
	proc.fs.lock().access_profile.set_gid(gid)?;
	Ok(0)
}

pub fn setregid(
	Args((rgid, egid)): Args<(c_int, c_int)>,
	ap: AccessProfile,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Validation
	if rgid < -1 || egid < -1 {
		return Err(errno!(EINVAL));
	}
	if !ap.is_privileged()
		&& (![-1, ap.gid as _, ap.egid as _].contains(&rgid)
			|| ![-1, ap.gid as _, ap.egid as _, ap.sgid as _].contains(&egid))
	{
		return Err(errno!(EPERM));
	}
	// Update
	let new_rgid = match rgid {
		-1 => ap.gid,
		i => i as _,
	};
	let new_egid = match egid {
		-1 => ap.egid,
		i => i as _,
	};
	let mut fs = proc.fs.lock();
	fs.access_profile.gid = new_rgid;
	fs.access_profile.egid = new_egid;
	if new_rgid != ap.gid || new_egid != ap.gid {
		fs.access_profile.sgid = new_egid;
	}
	Ok(0)
}

pub fn setresgid(
	Args((rgid, egid, sgid)): Args<(c_int, c_int, c_int)>,
	ap: AccessProfile,
	proc: Arc<Process>,
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
	let mut fs = proc.fs.lock();
	fs.access_profile.gid = match rgid {
		-1 => ap.gid,
		i => i as _,
	};
	fs.access_profile.egid = match egid {
		-1 => ap.egid,
		i => i as _,
	};
	fs.access_profile.sgid = match sgid {
		-1 => ap.sgid,
		i => i as _,
	};
	Ok(0)
}
