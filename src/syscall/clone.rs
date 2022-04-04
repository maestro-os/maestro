//! The `clone` system call creates a child process.

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `clone` syscall.
pub fn clone(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
