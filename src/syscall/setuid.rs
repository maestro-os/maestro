//! The `setuid` syscall sets the UID of the process's owner.

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `setuid` syscall.
pub fn setuid(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
