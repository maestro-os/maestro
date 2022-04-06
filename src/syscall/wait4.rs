//! The `wait4` system call waits for a process to change state.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::process::rusage::RUsage;
use super::waitpid;

/// The implementation of the `wait4` syscall.
pub fn wait4(regs: &Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let wstatus: SyscallPtr<i32> = (regs.ecx as usize).into();
	let options = regs.edx as i32;
	let rusage: SyscallPtr<RUsage> = (regs.esi as usize).into();

	waitpid::do_waitpid(pid, wstatus, options, Some(rusage))
}
