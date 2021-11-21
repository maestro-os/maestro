//! The `brk` system call is deprecated on this kernel. Thus it will always fail.

use crate::errno::Errno;
use crate::errno;
use crate::process::Regs;

/// The implementation of the `brk` syscall.
pub fn brk(_regs: &Regs) -> Result<i32, Errno> {
	Err(errno::ENOMEM)
}
