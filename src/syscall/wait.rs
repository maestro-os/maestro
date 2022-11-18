//! The `wait` system call is a simpler version of the `waitpid` system call.

use core::ffi::c_int;
use super::waitpid;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use macros::syscall;

/// The implementation of the `wait` syscall.
#[syscall]
pub fn wait(wstatus: SyscallPtr::<c_int>) -> Result<i32, Errno> {
	waitpid::do_waitpid(regs, -1, wstatus, waitpid::WEXITED, None)
}
