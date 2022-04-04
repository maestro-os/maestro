//! The `wait4` system call waits for a process to change state.

use core::ptr::NonNull;
use crate::errno::Errno;
use crate::process::regs::Regs;
use crate::process::rusage::RUsage;
use super::waitpid;

/// The implementation of the `wait4` syscall.
pub fn wait4(regs: &Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let wstatus = regs.ecx as *mut i32;
	let options = regs.edx as i32;
	let rusage = regs.edx as *mut RUsage;

	waitpid::do_waitpid(pid, NonNull::new(wstatus), options, NonNull::new(rusage))
}
