//! The `chown32` system call changes the owner of a file.

use crate::errno::Errno;
use crate::file::perm::{Gid, Uid};
use crate::process::mem_space::ptr::SyscallString;
use macros::syscall;

#[syscall]
pub fn chown32(pathname: SyscallString, owner: Uid, group: Gid) -> EResult<i32> {
	super::chown::do_chown(pathname, owner, group, true)
}
