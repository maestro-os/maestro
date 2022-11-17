//! The `wait` system call is a simpler version of the `waitpid` system call.

use super::waitpid;
use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;

/// The implementation of the `wait` syscall.
pub fn wait(regs: &Regs) -> Result<i32, Errno> {
	let wstatus: SyscallPtr<i32> = (regs.ebx as usize).into();
	waitpid::do_waitpid(regs, -1, wstatus, waitpid::WEXITED, None)
}
