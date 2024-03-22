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

use super::Mode;
use crate::file::File;
use utils::{errno, errno::EResult};

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

/// A set of informations determining whether an agent (example: a process) can access a resource.
///
/// Implementations of this structure may contain functions to check access to an object. Custom
/// implementations may be added.
///
/// Example:
/// ```rust
/// impl AccessProfile {
/// 	pub fn can_use(&self, obj: &Obj) -> bool {
/// 		// your implementation
/// 		// ...
/// 	}
/// }
/// ```
///
/// Fields of this structure are not directly accessible because mishandling them is prone to
/// cause privilege escalations. Instead, they should be modified only through the structure's
/// functions.
#[derive(Clone, Copy, Debug)]
pub struct AccessProfile {
	/// Real ID of user.
	uid: Uid,
	/// Real ID of group.
	gid: Gid,

	/// The effective ID of user.
	euid: Uid,
	/// The effective ID of group.
	egid: Gid,

	/// The saved user ID.
	suid: Uid,
	/// The saved group ID.
	sgid: Gid,
}

impl AccessProfile {
	/// Permissions to be used to access files being the kernel itself (or root user).
	pub const KERNEL: Self = Self {
		uid: 0,
		gid: 0,

		euid: 0,
		egid: 0,

		suid: 0,
		sgid: 0,
	};

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

	/// Extracts UID and GID from file and returns the associated profile.
	pub fn from_file(file: &File) -> Self {
		Self::new(file.get_uid(), file.get_gid())
	}

	/// Returns the real user ID.
	pub fn get_uid(&self) -> Uid {
		self.uid
	}

	/// Returns the effective user ID.
	pub fn get_euid(&self) -> Uid {
		self.euid
	}

	/// Returns the saved user ID.
	pub fn get_suid(&self) -> Uid {
		self.suid
	}

	/// Returns the real group ID.
	pub fn get_gid(&self) -> Gid {
		self.gid
	}

	/// Returns the effective group ID.
	pub fn get_egid(&self) -> Gid {
		self.egid
	}

	/// Returns the saved group ID.
	pub fn get_sgid(&self) -> Gid {
		self.sgid
	}

	/// Tells whether the agent is privileged (root).
	pub fn is_privileged(&self) -> bool {
		self.uid == ROOT_UID
			|| self.euid == ROOT_UID
			|| self.gid == ROOT_GID
			|| self.egid == ROOT_GID
	}

	/// Sets the user ID in the same way the `setgid` system call does.
	///
	/// If the agent is not privileged enough to make the change, the function returns an error.
	pub fn set_uid(&mut self, uid: Uid) -> EResult<()> {
		if self.euid == ROOT_UID {
			// privileged
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
			// privileged
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
