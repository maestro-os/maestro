//! UNIX permissions are detailed in the POSIX specification.
//!
//! This module implements management of such permissions.

use super::Mode;

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
///     pub fn can_use(&self, obj: &Obj) -> bool {
///         // your implementation
///         // ...
///     }
/// }
/// ```
///
/// Fields of this structure are not directly accessible because mishandling them is prone to
/// cause privilege escalations. Instead, they should be modified only through the structure's
/// functions.
#[derive(Clone, Copy)]
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
}
