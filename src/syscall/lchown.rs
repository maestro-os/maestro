//! The `lchown` system call changes the owner of a symbolic link file.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallString;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn lchown(pathname: SyscallString, owner: c_int, group: c_int) -> EResult<i32> {
	super::chown::do_chown(pathname, owner, group, false)
}
