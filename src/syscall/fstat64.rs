//! The `fstat64` system call allows get the status of a file.

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `fstat64` syscall.
pub fn fstat64(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
