//! The `wait4` system call waits for a process to change state.

use super::waitpid;
use crate::{
	errno::Errno,
	process::{mem_space::ptr::SyscallPtr, rusage::RUsage},
};
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn wait4(
	pid: c_int,
	wstatus: SyscallPtr<c_int>,
	options: c_int,
	rusage: SyscallPtr<RUsage>,
) -> Result<i32, Errno> {
	if rusage.is_null() {
		waitpid::do_waitpid(regs, pid, wstatus, options | waitpid::WEXITED, None)
	} else {
		waitpid::do_waitpid(regs, pid, wstatus, options | waitpid::WEXITED, Some(rusage))
	}
}
