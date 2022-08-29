//! The `setgid` syscall sets the GID of the process's owner.

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `setgid` syscall.
pub fn setgid(_regs: &Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
