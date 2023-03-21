//! The `setgid` syscall sets the GID of the process's owner.

use crate::errno::Errno;
use crate::file::Gid;
use macros::syscall;

#[syscall]
pub fn setgid(_gid: Gid) -> Result<i32, Errno> {
	// TODO
	todo!();
}
