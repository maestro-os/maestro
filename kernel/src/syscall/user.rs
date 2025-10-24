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
	memory::user::{UserPtr, UserSlice},
	process::Process,
};
use core::{ffi::c_int, hint::unlikely};
use utils::{errno, errno::EResult};

pub fn getuid() -> EResult<usize> {
	Ok(AccessProfile::cur_task().uid as _)
}

pub fn geteuid() -> EResult<usize> {
	Ok(AccessProfile::cur_task().euid as _)
}

pub fn getresuid(ruid: UserPtr<Uid>, euid: UserPtr<Uid>, suid: UserPtr<Uid>) -> EResult<usize> {
	let ap = AccessProfile::cur_task();
	ruid.copy_to_user(&ap.uid)?;
	euid.copy_to_user(&ap.euid)?;
	suid.copy_to_user(&ap.suid)?;
	Ok(0)
}

pub fn getgid() -> EResult<usize> {
	Ok(AccessProfile::cur_task().gid as _)
}

pub fn getegid() -> EResult<usize> {
	Ok(AccessProfile::cur_task().egid as _)
}

pub fn getresgid(rgid: UserPtr<Gid>, egid: UserPtr<Gid>, sgid: UserPtr<Gid>) -> EResult<usize> {
	let ap = AccessProfile::cur_task();
	rgid.copy_to_user(&ap.gid)?;
	egid.copy_to_user(&ap.egid)?;
	sgid.copy_to_user(&ap.sgid)?;
	Ok(0)
}

pub fn setuid(uid: Uid) -> EResult<usize> {
	Process::current().fs().lock().access_profile.set_uid(uid)?;
	Ok(0)
}

pub fn setreuid(ruid: c_int, euid: c_int) -> EResult<usize> {
	// Validation
	if ruid < -1 || euid < -1 {
		return Err(errno!(EINVAL));
	}
	let ap = AccessProfile::cur_task();
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
	let proc = Process::current();
	let mut fs = proc.fs().lock();
	fs.access_profile.uid = new_ruid;
	fs.access_profile.euid = new_euid;
	if new_ruid != ap.uid || new_euid != ap.uid {
		fs.access_profile.suid = new_euid;
	}
	Ok(0)
}

pub fn setresuid(ruid: c_int, euid: c_int, suid: c_int) -> EResult<usize> {
	// Validation
	if ruid < -1 || euid < -1 || suid < -1 {
		return Err(errno!(EINVAL));
	}
	let ap = AccessProfile::cur_task();
	if !ap.is_privileged() {
		let allowed = [-1, ap.uid as _, ap.euid as _, ap.suid as _];
		if !allowed.contains(&ruid) || !allowed.contains(&euid) || !allowed.contains(&suid) {
			return Err(errno!(EPERM));
		}
	}
	// Update
	let proc = Process::current();
	let mut fs = proc.fs().lock();
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

pub fn setgid(gid: Gid) -> EResult<usize> {
	Process::current().fs().lock().access_profile.set_gid(gid)?;
	Ok(0)
}

pub fn setregid(rgid: c_int, egid: c_int) -> EResult<usize> {
	// Validation
	if rgid < -1 || egid < -1 {
		return Err(errno!(EINVAL));
	}
	let ap = AccessProfile::cur_task();
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
	let proc = Process::current();
	let mut fs = proc.fs().lock();
	fs.access_profile.gid = new_rgid;
	fs.access_profile.egid = new_egid;
	if new_rgid != ap.gid || new_egid != ap.gid {
		fs.access_profile.sgid = new_egid;
	}
	Ok(0)
}

pub fn setresgid(rgid: c_int, egid: c_int, sgid: c_int) -> EResult<usize> {
	// Validation
	if rgid < -1 || egid < -1 || sgid < -1 {
		return Err(errno!(EINVAL));
	}
	let ap = AccessProfile::cur_task();
	if !ap.is_privileged() {
		let allowed = [-1, ap.gid as _, ap.egid as _, ap.sgid as _];
		if !allowed.contains(&rgid) || !allowed.contains(&egid) || !allowed.contains(&sgid) {
			return Err(errno!(EPERM));
		}
	}
	// Update
	let proc = Process::current();
	let mut fs = proc.fs().lock();
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

pub fn getgroups(size: c_int, list: *mut Gid) -> EResult<usize> {
	let proc = Process::current();
	let fs = proc.fs().lock();
	if size > 0 {
		if size as usize != fs.groups.len() {
			return Err(errno!(EINVAL));
		}
		let list = UserSlice::from_user(list, size as _)?;
		list.copy_to_user(0, &fs.groups)?;
	}
	Ok(fs.groups.len())
}

pub fn getgroups32(size: c_int, list: *mut Gid) -> EResult<usize> {
	getgroups(size, list)
}

pub fn setgroups(size: usize, list: *mut Gid) -> EResult<usize> {
	let proc = Process::current();
	let mut fs = proc.fs().lock();
	if unlikely(!fs.access_profile.is_privileged()) {
		return Err(errno!(EPERM));
	}
	let list = UserSlice::from_user(list, size)?;
	// TODO no need to zero-init
	fs.groups.resize(size, 0)?;
	list.copy_from_user(0, &mut fs.groups)?;
	Ok(0)
}

pub fn setgroups32(size: usize, list: *mut Gid) -> EResult<usize> {
	setgroups(size, list)
}
