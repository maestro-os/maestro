//! The `brk` is a deprecated system call on this kernel. Thus it will always fail.

use crate::errno::Errno;
use crate::errno;
use crate::util;

/// The implementation of the `brk` syscall.
pub fn brk(_regs: &util::Regs) -> Result<i32, Errno> {
	return Err(errno::ENOMEM);
}
