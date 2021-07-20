//! The `setgid` syscall sets the GID of the process's owner.

use crate::errno::Errno;
use crate::util;

/// The implementation of the `setgid` syscall.
pub fn setgid(_regs: &util::Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
