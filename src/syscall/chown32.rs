//! The `chown32` system call changes the owner of a file.

use crate::{errno::Errno, process::mem_space::ptr::SyscallString};
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn chown32(pathname: SyscallString, owner: c_int, group: c_int) -> EResult<i32> {
	super::chown::do_chown(pathname, owner, group, true)
}
