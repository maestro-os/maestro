//! The `fadvise64_64` syscall gives hints to the kernel about file accesses.

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `setgid32` syscall.
pub fn fadvise64_64(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	Ok(0)
}
