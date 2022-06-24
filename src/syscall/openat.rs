//! The `openat` syscall allows to open a file.

use crate::errno::Errno;
use crate::file;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `openat` syscall.
pub fn openat(regs: &Regs) -> Result<i32, Errno> {
	let _dirfd = regs.ebx as i32;
	let _pathname: SyscallString = (regs.ecx as usize).into();
	let _flags = regs.edx as i32;
	let _mode = regs.esi as file::Mode;

	// TODO
	todo!();
}
