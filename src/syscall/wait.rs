//! The `wait` system call is a simpler version of the `waitpid` system call.

use core::ptr::NonNull;
use crate::errno::Errno;
use crate::process::regs::Regs;
use super::waitpid;

/// The implementation of the `wait` syscall.
pub fn wait(regs: &Regs) -> Result<i32, Errno> {
	let wstatus = regs.ebx as *mut i32;
	waitpid::do_waitpid(-1, NonNull::new(wstatus), 0, None)
}
