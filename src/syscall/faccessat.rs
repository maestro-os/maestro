//! The `faccessat` system call allows to check access to a given file.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `faccessat` syscall.
pub fn faccessat(regs: &Regs) -> Result<i32, Errno> {
	let dir_fd = regs.ebx as i32;
	let pathname: SyscallString = (regs.ecx as usize).into();
	let mode = regs.edx as i32;

	super::access::do_access(Some(dir_fd), pathname, mode, None)
}
