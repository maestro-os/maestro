//! The `wait` system call is a simpler version of the `waitpid` system call.

use super::waitpid;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn wait(wstatus: SyscallPtr<c_int>) -> Result<i32, Errno> {
	waitpid::do_waitpid(regs, -1, wstatus, waitpid::WEXITED, None)
}
