//! The `setuid` syscall sets the UID of the process's owner.

use crate::errno::Errno;
use crate::file::Uid;
use macros::syscall;

/// The implementation of the `setuid` syscall.
#[syscall]
pub fn setuid(uid: Uid) -> Result<i32, Errno> {
	// TODO
	todo!();
}
