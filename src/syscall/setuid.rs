//! The `setuid` syscall sets the UID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `setuid` syscall.
pub fn setuid(_proc: &mut Process, _regs: &util::Regs) -> Result<i32, Errno> {
	// TODO
	todo!();
}
