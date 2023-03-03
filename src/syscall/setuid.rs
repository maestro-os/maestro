//! The `setuid` syscall sets the UID of the process's owner.

use crate::errno::Errno;
use crate::file::Uid;
use macros::syscall;

#[syscall]
pub fn setuid(_uid: Uid) -> Result<i32, Errno> {
	// TODO
	todo!();
}
