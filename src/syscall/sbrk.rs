//! The `sbrk` system call is deprecated on this kernel. Thus it will always fail.

use crate::errno::Errno;
use crate::errno;
use crate::process::Regs;

/// The implementation of the `sbrk` syscall.
pub fn sbrk(_regs: &Regs) -> Result<i32, Errno> {
	Err(errno::ENOMEM)
}
