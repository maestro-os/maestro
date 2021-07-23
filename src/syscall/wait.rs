//! The `wait` system call is a simpler version of the `waitpid` system call.

use crate::errno::Errno;
use crate::util;
use super::waitpid;

/// The implementation of the `wait` syscall.
pub fn wait(regs: &util::Regs) -> Result<i32, Errno> {
	let wstatus = regs.ebx as *mut i32;

	waitpid::do_waitpid(-1, wstatus, 0)
}
