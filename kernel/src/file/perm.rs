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

//! UNIX permissions are detailed in the POSIX specification.
//!
//! This module implements management of such permissions.

use super::{FileType, Mode, Stat, vfs};
use crate::process::Process;
use utils::{TryClone, collections::vec::Vec, errno, errno::EResult, ptr::arc::Arc};

/// Type representing a user ID.
pub type Uid = u16;
/// Type representing a group ID.
pub type Gid = u16;

/// The root user ID.
pub const ROOT_UID: Uid = 0;
/// The root group ID.
pub const ROOT_GID: Gid = 0;

/// User: Read, Write and Execute.
pub const S_IRWXU: Mode = 0o0700;
/// User: Read.
pub const S_IRUSR: Mode = 0o0400;
/// User: Write.
pub const S_IWUSR: Mode = 0o0200;
/// User: Execute.
pub const S_IXUSR: Mode = 0o0100;
/// Group: Read, Write and Execute.
pub const S_IRWXG: Mode = 0o0070;
/// Group: Read.
pub const S_IRGRP: Mode = 0o0040;
/// Group: Write.
pub const S_IWGRP: Mode = 0o0020;
/// Group: Execute.
pub const S_IXGRP: Mode = 0o0010;
/// Other: Read, Write and Execute.
pub const S_IRWXO: Mode = 0o0007;
/// Other: Read.
pub const S_IROTH: Mode = 0o0004;
/// Other: Write.
pub const S_IWOTH: Mode = 0o0002;
/// Other: Execute.
pub const S_IXOTH: Mode = 0o0001;
/// Setuid.
pub const S_ISUID: Mode = 0o4000;
/// Setgid.
pub const S_ISGID: Mode = 0o2000;
/// Sticky bit.
pub const S_ISVTX: Mode = 0o1000;

/// A set of information determining whether a process can access a resource.
///
/// Fields of this structure are not directly accessible because mishandling them is prone to
/// cause privilege escalations. Instead, they should be modified only through the structure's
/// functions.
#[derive(Clone, Copy, Debug)]
pub struct AccessProfile {
	/// Real ID of user
	pub uid: Uid,
	/// Real ID of group
	pub gid: Gid,

	/// The effective ID of user
	pub euid: Uid,
	/// The effective ID of group
	pub egid: Gid,

	/// The saved user ID
	pub suid: Uid,
	/// The saved group ID
	pub sgid: Gid,
}

impl AccessProfile {
	/// Creates a profile from the given IDs.
	pub fn new(uid: Uid, gid: Gid) -> Self {
		Self {
			uid,
			gid,

			euid: uid,
			egid: gid,

			suid: uid,
			sgid: gid,
		}
	}

	/// Returns a copy of the current process's instance.
	pub fn current() -> Self {
		match &Process::current().fs {
			Some(fs) => fs.lock().ap,
			None => Self::new(ROOT_UID, ROOT_GID),
		}
	}

	/// Sets the user ID in the same way the `setgid` system call does.
	///
	/// If the agent is not privileged enough to make the change, the function returns an error.
	pub fn set_uid(&mut self, uid: Uid) -> EResult<()> {
		if self.euid == ROOT_UID {
			// Privileged
			self.uid = uid;
			self.euid = uid;
			self.suid = uid;
			Ok(())
		} else if uid == self.uid || uid == self.euid || uid == self.suid {
			self.euid = uid;
			Ok(())
		} else {
			Err(errno!(EPERM))
		}
	}

	/// Sets the effective user ID.
	///
	/// If the agent is not privileged enough to make the change, the function returns an error.
	pub fn set_euid(&mut self, uid: Uid) -> EResult<()> {
		if uid == ROOT_UID || uid == self.uid || uid == self.euid || uid == self.suid {
			self.euid = uid;
			Ok(())
		} else {
			Err(errno!(EPERM))
		}
	}

	/// Sets the group ID in the way the `setgid` system call does.
	///
	/// If the agent is not privileged enough to make the change, the function returns an error.
	pub fn set_gid(&mut self, gid: Gid) -> EResult<()> {
		if self.egid == ROOT_GID {
			// Privileged
			self.gid = gid;
			self.egid = gid;
			self.sgid = gid;
			Ok(())
		} else if gid == self.gid || gid == self.egid || gid == self.sgid {
			self.egid = gid;
			Ok(())
		} else {
			Err(errno!(EPERM))
		}
	}

	/// Sets the effective group ID.
	///
	/// If the agent is not privileged enough to make the change, the function returns an error.
	pub fn set_egid(&mut self, gid: Uid) -> EResult<()> {
		if gid == ROOT_GID || gid == self.gid || gid == self.egid || gid == self.sgid {
			self.egid = gid;
			Ok(())
		} else {
			Err(errno!(EPERM))
		}
	}
}

/// A process's filesystem access information.
pub struct ProcessFs {
	/// The process's access profile, containing user and group IDs.
	pub ap: AccessProfile,
	/// Supplementary group IDs
	pub groups: Vec<Gid>,

	/// Current working directory
	///
	/// If `None`, using the root directory of the VFS.
	pub cwd: Arc<vfs::Entry>,
	/// Current root path used by the process
	///
	/// If `None`, using the root directory of the VFS.
	pub chroot: Arc<vfs::Entry>,
}

impl TryClone for ProcessFs {
	fn try_clone(&self) -> Result<Self, Self::Error> {
		Ok(Self {
			ap: self.ap,
			groups: self.groups.try_clone()?,

			cwd: self.cwd.clone(),
			chroot: self.chroot.clone(),
		})
	}
}

/// Tells whether the current process is privileged (root).
pub fn is_privileged() -> bool {
	let ap = AccessProfile::current();
	ap.euid == ROOT_UID || ap.egid == ROOT_GID
}

#[inline]
fn match_ids(stat: &Stat, effective: bool) -> (bool, bool) {
	let proc = Process::current();
	let Some(fs) = &proc.fs else {
		return (true, true);
	};
	let fs = fs.lock();
	let (uid, gid) = if effective {
		(fs.ap.euid, fs.ap.egid)
	} else {
		(fs.ap.uid, fs.ap.gid)
	};
	(
		stat.uid == uid,
		stat.gid == gid || fs.groups.contains(&stat.gid),
	)
}

/// Tells whether the current process can read a file with the given status.
///
/// `effective` tells whether to use effective IDs. If not, real IDs are used.
pub fn can_read_file(stat: &Stat, effective: bool) -> bool {
	if is_privileged() {
		return true;
	}
	let (uid, gid) = match_ids(stat, effective);
	if stat.mode & S_IRUSR != 0 && uid {
		return true;
	}
	if stat.mode & S_IRGRP != 0 && gid {
		return true;
	}
	stat.mode & S_IROTH != 0
}

/// Tells whether the agent can list files of a directory with the given status, **not**
/// including access to files' contents and metadata.
#[inline]
pub fn can_list_directory(stat: &Stat) -> bool {
	can_read_file(stat, true)
}

/// Tells whether the agent can write a file with the given status.
///
/// `effective` tells whether to use effective IDs. If not, real IDs are used.
pub fn can_write_file(stat: &Stat, effective: bool) -> bool {
	if is_privileged() {
		return true;
	}
	let (uid, gid) = match_ids(stat, effective);
	if stat.mode & S_IWUSR != 0 && uid {
		return true;
	}
	if stat.mode & S_IWGRP != 0 && gid {
		return true;
	}
	stat.mode & S_IWOTH != 0
}

/// Tells whether the agent can modify entries in a directory with the given status, including
/// creating files, deleting files, and renaming files.
#[inline]
pub fn can_write_directory(stat: &Stat) -> bool {
	can_write_file(stat, true) && can_execute_file(stat, true)
}

/// Tells whether the agent can execute a file with the given status.
///
/// `effective` tells whether to use effective IDs. If not, real IDs are used.
pub fn can_execute_file(stat: &Stat, effective: bool) -> bool {
	// If root, bypass checks (unless the file is a regular file)
	if stat.get_type() != Some(FileType::Regular) && is_privileged() {
		return true;
	}
	let (uid, gid) = match_ids(stat, effective);
	if stat.mode & S_IXUSR != 0 && uid {
		return true;
	}
	if stat.mode & S_IXGRP != 0 && gid {
		return true;
	}
	stat.mode & S_IXOTH != 0
}

/// Tells whether the current process can access files of a directory with the given status, *if
/// the name of the file is known*.
#[inline]
pub fn can_search_directory(stat: &Stat) -> bool {
	can_execute_file(stat, true)
}

/// Tells whether the current process can set permissions for a file with the given status.
pub fn can_set_file_permissions(stat: &Stat) -> bool {
	let ap = AccessProfile::current();
	ap.uid == ROOT_UID || ap.uid == stat.uid
}

/// Tells whether the current process can kill `proc`.
pub fn can_kill(proc: &Process) -> bool {
	if is_privileged() {
		return true;
	}
	let ap = AccessProfile::current();
	let other_ap = proc.fs().lock().ap;
	// if sender's `uid` or `euid` equals receiver's `uid` or `suid`
	ap.uid == other_ap.uid
		|| ap.uid == other_ap.suid
		|| ap.euid == other_ap.uid
		|| ap.euid == other_ap.suid
}
