//! The `nanosleep` system call allows to TODO.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::time::unit::Timespec;

/// The implementation of the `nanosleep` syscall.
pub fn nanosleep(regs: &Regs) -> Result<i32, Errno> {
	let _req: SyscallPtr<Timespec> = (regs.ebx as usize).into();
	let _rem: SyscallPtr<Timespec> = (regs.ecx as usize).into();

	// TODO
	Ok(0)
}
