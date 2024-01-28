//! The `faccessat` system call allows to check access to a given file.

use crate::{errno::Errno, process::mem_space::ptr::SyscallString};
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn faccessat(dir_fd: c_int, pathname: SyscallString, mode: c_int) -> Result<i32, Errno> {
	super::access::do_access(Some(dir_fd), pathname, mode, None)
}
