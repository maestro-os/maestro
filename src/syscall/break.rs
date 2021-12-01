//! The `break` system call is not implemented.

use crate::errno::Errno;
use crate::errno;
use crate::process::Regs;

/// The implementation of the `break` syscall.
pub fn r#break(_regs: &Regs) -> Result<i32, Errno> {
	Err(errno::ENOSYS)
}
